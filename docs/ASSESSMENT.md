# Đánh giá baseline và hướng cải tạo

Ngày kiểm tra: 2026-09-13. Phạm vi: toàn bộ 9 file authored ban đầu (README, CHANGELOG, năm docs, prompt, schema); không coi Git metadata là source ứng dụng. HEAD ban đầu: `151c1905603bbbfa4e01cd0280f5a68bf6ae58a6`, nhánh `master`, không có file tracked. Toàn bộ tài liệu/schema đã tồn tại ở dạng untracked trước lần chỉnh này. Vì vậy HEAD không đại diện nội dung được audit; evidence dưới đây chỉ baseline working tree. Bản sao trước sửa được giữ tạm tại `/tmp/camapro-baseline-j21uc5h6` trong phiên thực hiện, không phải artifact portable.

## Kết luận

**Ý tưởng/stack có hướng đúng; chưa phải một project phần mềm có thể build hoặc đánh giá chất lượng runtime.** Không có Android/desktop source, dependency manifest, lockfile, test hay CI. Điểm cần cải tạo là hợp đồng, thứ tự kiểm chứng và quy trình agent, không phải rewrite code chưa tồn tại.

Giữ: local-first, một Camera2 owner, Rust/GStreamer core, UI không nhận raw frames, capability-driven controls, bounded queues, platform adapters. Không thêm daemon/multi-agent platform/microservices hoặc scaffold hàng loạt module tương lai.

## Phát hiện trên tài liệu ban đầu

Các line dưới đây thuộc **bản trước sửa**, không phải line hiện tại.

| Mức | Bằng chứng baseline | Vấn đề | Cải tạo / gate |
|---|---|---|---|
| Cao | `docs/ROADMAP.md:18–53`, `prompts/IMPLEMENT.md:83–103` | cho chạy real LAN trước pairing/watchdog ở P5; cleanup/trust là điều kiện nền, không phần trang trí | G2 có auth cả control/media + lease, D03 cần duyệt trước release |
| Cao | `docs/ARCHITECTURE.md:178`, `docs/DEVELOPMENT.md:113–121` | token-only trước, mã hóa “future”; không xác định bảo vệ stream và identity binding | trust flow đề xuất trong protocol, gate TLS/pinning/storage ở G0; không tuyên bố đã bảo mật |
| Cao | `protocol/control-message.schema.json:6–37` | chỉ bắt buộc v/type; camera.set không payload/id và response/error thiếu dữ liệu vẫn không bị required tương ứng ngăn chặn | giữ nguyên và gắn nhãn draft; G1 typed variants + fixtures + Kotlin/Rust parity |
| Cao | `docs/ARCHITECTURE.md:62,140–159` | không raw frames qua React nhưng không có native surface ownership hoặc phương án Wayland | G0 preview spike bắt buộc; không lách bằng base64/JPEG bridge |
| Cao | `docs/ROADMAP.md:24`, `docs/ARCHITECTURE.md:145` | MJPEG 1080p30 được đặt làm target nhưng chưa có hardware/codec feasibility proof | benchmark 720p30 trước, target release 1080p30 có reference device và stop condition |
| Vừa | `docs/ARCHITECTURE.md:182–200` | sơ đồ trạng thái thiếu cancel/failure cleanup, Error không có recovery, reconnect chưa có consent policy | bảng transition + generation + lease + đề xuất Ready sau reconnect |
| Vừa | `docs/PROTOCOL.md:16,61–99` | thiếu direction, units, atomic sensor validation, replay/request completion | thiết kế wire semantics rõ, semantic fixtures ngoài JSON Schema |
| Vừa | `docs/PLATFORMS.md:76–88` | “Media Foundation virtual camera” chưa đủ để suy ra OS/API/package support | D08 và Windows feasibility trước lời hứa hỗ trợ; G8 real Windows proof |
| Vừa | `docs/ARCHITECTURE.md:204–226`, `docs/PLATFORMS.md:59–74` | cùng CommandRegistry chưa giải thích CLI truy cập process nào | một command authority; đề xuất IPC, không dựng daemon lúc này |
| Vừa | `docs/DEVELOPMENT.md:18–42,123–135` | command chưa tồn tại; checklist dùng dấu ✓ có thể bị đọc như đã hoàn thành; all-features dễ xung đột platform | commands tương lai có cwd + nhãn rõ; checklist unchecked; native build riêng |
| Vừa | `prompts/IMPLEMENT.md:29–79` và toàn repo | prompt lặp canon, không entrypoint/routing skills/ownership | AGENTS.md ngắn, prompt handoff, 4 project-local skills đọc thủ công |

## Phân biệt kết quả lần này

Đã cải tạo **tài liệu và cơ chế hướng dẫn agent**, không phải ứng dụng. `docs/ROADMAP.md` là roadmap duy nhất; decision ledger phân biệt retained/proposed/open. Mọi gate ứng dụng vẫn not started. Schema được giữ nguyên có chủ ý: đổi hợp đồng cần G1 tests/type consumers, không chỉnh JSON cho có vẻ hoàn thiện.

Chưa chứng minh: Camera2 FPS, native preview Wayland, bảo mật runtime, TLS/GStreamer compatibility, Windows API floor, virtual-camera consumers, bất kỳ con số latency/memory hoặc build nào. Đây là blockers/evidence gates cụ thể, không phải lỗi runtime đã quan sát.

## Kiểm chứng schema đã chạy

Dùng `jsonschema==4.23.0` trong virtualenv tạm `/tmp/camapro-schema-review-venv` (không cài vào hệ thống/project). `Draft202012Validator.check_schema` pass, nhưng validator thực sự chấp nhận cả 5 mẫu không đủ hợp đồng: camera.set thiếu id/payload; response thiếu id/kết quả; error thiếu error body; ISO âm không được kiểm tra; response vừa ok=true vừa có error. Hai negative controls (major version lạ, command lạ) bị từ chối đúng. Kết quả này tái hiện độ lỏng của schema, **không phải** chứng nhận protocol đạt yêu cầu.

## Cần quyết định tiếp

G0 phải xác định điện thoại Android/reference mode, native preview, TLS/pinning/storage, Android SDK/OS floors và chính sách resume. Sau khi có evidence và owner approval, triển khai G1 rồi G2. Không cần chọn thêm agent framework; lead + bounded worker + independent reviewer đủ cho repo này.
