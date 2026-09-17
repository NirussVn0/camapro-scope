import QRCode from "qrcode";

/**
 * Generate standard QR code SVG string for pairing payload.
 */
export async function generateQrSvg(text: string, pixelSize = 200): Promise<string> {
  return QRCode.toString(text, {
    type: "svg",
    width: pixelSize,
    margin: 1,
    color: {
      dark: "#000000",
      light: "#ffffff",
    },
  });
}

/**
 * Generate standard QR code Data URL (PNG) for pairing payload.
 */
export async function generateQrDataUrl(text: string, pixelSize = 200): Promise<string> {
  return QRCode.toDataURL(text, {
    width: pixelSize,
    margin: 1,
    color: {
      dark: "#000000",
      light: "#ffffff",
    },
  });
}
