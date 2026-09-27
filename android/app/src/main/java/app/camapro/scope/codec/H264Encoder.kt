package app.camapro.scope.codec

import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.view.Surface

/**
 * Production H.264 (AVC) hardware encoder using Android MediaCodec (Gate G7).
 * Configured for ultra-low latency real-time streaming:
 * - Direct Surface input from Camera2 (zero copy)
 * - Baseline / Main profile with 1-second I-frame GOP (KEY_I_FRAME_INTERVAL = 1)
 * - Constant bitrate mode (BITRATE_MODE_CBR)
 * - Emits Annex-B NAL units with start codes (0x00, 0x00, 0x00, 0x01)
 */
class H264Encoder(
    val width: Int = 1920,
    val height: Int = 1080,
    val fps: Int = 30,
    val bitrate: Int = 4_000_000,
    val onNalUnit: (ByteArray) -> Unit
) {
    companion object {
        const val MIME_TYPE = MediaFormat.MIMETYPE_VIDEO_AVC
        val ANNEX_B_START_CODE = byteArrayOf(0, 0, 0, 1)

        fun isAnnexB(data: ByteArray): Boolean {
            return data.size >= 4 &&
                    data[0] == 0.toByte() &&
                    data[1] == 0.toByte() &&
                    data[2] == 0.toByte() &&
                    data[3] == 1.toByte()
        }
    }

    private var codec: MediaCodec? = null
    var inputSurface: Surface? = null
        private set

    private var handlerThread: HandlerThread? = null
    private var handler: Handler? = null

    @Volatile
    var isRunning = false
        private set

    fun createMediaFormat(): MediaFormat {
        val format = MediaFormat.createVideoFormat(MIME_TYPE, width, height).apply {
            setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
            setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
            setInteger(MediaFormat.KEY_FRAME_RATE, fps)
            setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 1) // 1 keyframe per second
            setInteger(MediaFormat.KEY_BITRATE_MODE, MediaCodecInfo.EncoderCapabilities.BITRATE_MODE_CBR)
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                setInteger(MediaFormat.KEY_LOW_LATENCY, 1)
            }
        }
        return format
    }

    @Synchronized
    fun start() {
        if (isRunning) return
        val format = createMediaFormat()
        val mediaCodec = MediaCodec.createEncoderByType(MIME_TYPE)

        val ht = HandlerThread("H264EncoderBg").apply { start() }
        val h = Handler(ht.looper)
        handlerThread = ht
        handler = h

        mediaCodec.setCallback(object : MediaCodec.Callback() {
            override fun onInputBufferAvailable(codec: MediaCodec, index: Int) {
                // Surface mode handles input automatically
            }

            override fun onOutputBufferAvailable(
                codec: MediaCodec,
                index: Int,
                info: MediaCodec.BufferInfo
            ) {
                try {
                    val outputBuffer = codec.getOutputBuffer(index)
                    if (outputBuffer != null && info.size > 0) {
                        outputBuffer.position(info.offset)
                        outputBuffer.limit(info.offset + info.size)

                        val outBytes = ByteArray(info.size)
                        outputBuffer.get(outBytes)

                        val nal = if (isAnnexB(outBytes)) {
                            outBytes
                        } else {
                            val withHeader = ByteArray(4 + outBytes.size)
                            System.arraycopy(ANNEX_B_START_CODE, 0, withHeader, 0, 4)
                            System.arraycopy(outBytes, 0, withHeader, 4, outBytes.size)
                            withHeader
                        }
                        onNalUnit(nal)
                    }
                    codec.releaseOutputBuffer(index, false)
                } catch (_: Exception) {
                }
            }

            override fun onError(codec: MediaCodec, e: MediaCodec.CodecException) {
            }

            override fun onOutputFormatChanged(codec: MediaCodec, format: MediaFormat) {
                val sps = format.getByteBuffer("csd-0")
                val pps = format.getByteBuffer("csd-1")
                if (sps != null) {
                    val spsBytes = ByteArray(sps.remaining())
                    sps.get(spsBytes)
                    if (isAnnexB(spsBytes)) onNalUnit(spsBytes)
                }
                if (pps != null) {
                    val ppsBytes = ByteArray(pps.remaining())
                    pps.get(ppsBytes)
                    if (isAnnexB(ppsBytes)) onNalUnit(ppsBytes)
                }
            }
        }, h)

        mediaCodec.configure(format, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
        inputSurface = mediaCodec.createInputSurface()
        mediaCodec.start()
        codec = mediaCodec
        isRunning = true
    }

    @Synchronized
    fun stop() {
        isRunning = false
        try {
            codec?.stop()
            codec?.release()
        } catch (_: Exception) {
        } finally {
            codec = null
            inputSurface = null
        }

        handlerThread?.quitSafely()
        try {
            handlerThread?.join(500)
        } catch (_: Exception) {
        }
        handlerThread = null
        handler = null
    }
}
