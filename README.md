# Camoufox Manager

**Camoufox Manager** (thư mục `manager/`) là ứng dụng macOS **local-first** để quản lý nhiều profile trình duyệt [Camoufox](https://github.com/daijro/camoufox) (Firefox anti-detect): tạo profile, gán proxy riêng, mở/đóng browser, xem fingerprint và dữ liệu profile. Mọi thứ nằm trên máy bạn, không cần tài khoản cloud.

Web UI và proxy trung gian chỉ lắng nghe trên `127.0.0.1`.

## Tính năng

- Tạo/xoá profile; fingerprint sinh **một lần** lúc tạo (theo OS bạn chọn) và giữ nguyên mỗi lần mở.
- Gán/bỏ gán proxy cho từng profile, kiểm tra IP, quốc gia, múi giờ, tọa độ của proxy.
- Khi Start, tự áp timezone, geolocation và IP WebRTC theo proxy.
- Start/Stop profile; cookie và đăng nhập được giữ trong profile Firefox persistent.
- Mở nhanh thư mục dữ liệu của profile; sao chép bảng dạng TSV để dán vào Excel/Sheets.
- Chạy dưới dạng app macOS hoặc server local không cửa sổ.

## Mô hình bảo mật

```text
Camoufox  ──►  127.0.0.1:31000+  ──►  Upstream proxy
(không có credential)   (Camoufox Manager thêm Proxy-Authorization)
```

- Mỗi upstream proxy ánh xạ sang một cổng local từ `31000`; browser chỉ thấy `127.0.0.1:<port>`, user/pass nằm trong tiến trình app.
- Fail-closed: upstream lỗi thì kết nối bị đóng, không đi thẳng bằng IP thật.
- Không commit `proxies.txt` hay `.env` vào Git.

## Yêu cầu

- macOS 11 trở lên, Rust toolchain (build từ source);
- [uv](https://docs.astral.sh/uv/) để cài Camoufox (tuỳ chọn, xem dưới);
- danh sách HTTP proxy, mỗi dòng `ip:port:user:password`.

## Cài Camoufox (tuỳ chọn)

Camoufox không bắt buộc để build và mở app: nếu chưa cài, app vẫn khởi động, báo "chưa cài" kèm gợi ý và tắt nút tạo/mở profile. Để cài:

```bash
manager/camoufox/setup.sh
```

Script tạo venv riêng, cài `camoufox` bản cố định và tải trình duyệt Camoufox (tổng khoảng 2,4 GB, vào `~/Library/Caches/camoufox`).

## Cấu hình

Proxy lưu trong cơ sở dữ liệu SQLite của app. Để thêm proxy (trước khi có trang Proxy library đầy đủ),
đặt file `~/Library/Application Support/camoufox-manager/proxies.txt` rồi mở lại app: các dòng mới
được nhập vào DB (trùng thì bỏ qua) và file được đổi tên thành `proxies.txt.bak`.

```text
127.0.0.1:8080:username:password
127.0.0.2:8080:username:password
```

Biến môi trường (đặt trong `~/Library/Application Support/camoufox-manager/.env` hoặc shell):

| Biến | Mặc định | Ý nghĩa |
| --- | --- | --- |
| `PROXY_FILE` | `<thư mục dữ liệu>/proxies.txt` | File proxy cũ, chỉ dùng cho lần nhập dữ liệu đầu tiên |
| `CM_PORT` | `7878` | Cổng UI/API (`serve`; cửa sổ app dùng cổng ngẫu nhiên nếu không đặt) |
| `CM_PROXY_BASE_PORT` | `31000` | Cổng local proxy đầu tiên (proxy #N dùng `base + N - 1`) |
| `CAMOUFOX_PYTHON` | `<thư mục dữ liệu>/camoufox/venv/bin/python` | Python của venv Camoufox |
| `CAMOUFOX_HEADLESS` | tắt | `1` để mở browser không cửa sổ |

## Chạy từ source

```bash
cd manager
cargo run --release            # cửa sổ app
cargo run --release -- serve   # headless, mở http://127.0.0.1:7878
```

Khi Start, cột "Playwright (ws)" là endpoint WebSocket theo **giao thức Playwright** (không phải CDP). Ví dụ Python: `browser = playwright.firefox.connect(ws); page = browser.contexts[0].pages[0]`; `contexts[0]` chính là profile persistent.

## Đóng gói

```bash
cd manager
./packaging/bundle-macos.sh    # -> manager/dist/Camoufox Manager.app
```

`.env` và dữ liệu không nằm trong `.app`; app đọc từ thư mục dữ liệu bên dưới.

## Dữ liệu lưu ở đâu

Tất cả trong `~/Library/Application Support/camoufox-manager/`:

| Dữ liệu | Vị trí |
| --- | --- |
| Profile, proxy, gán proxy, cài đặt | `camoufox-manager.db` (SQLite) |
| Biến môi trường | `.env` |
| Dữ liệu browser của profile | `camoufox/profiles/<profile-id>/user-data/` |
| Helper Python (app tự ghi) | `camoufox/helper/` |
| Python venv | `camoufox/venv` |

## Cấu trúc mã

`manager/` là Cargo workspace (xem `tasks/CM-1-rust-profile-manager/plan.md`):

```text
manager/
  crates/cm-core/            kiểu dữ liệu + trait (Engine, *Repo, ProxyChecker, Clock), không I/O
  crates/cm-store/           SQLite: repo, migration (src/migrations/), nhập dữ liệu cũ
  crates/cm-relay/           local proxy fail-closed + kiểm tra proxy
  crates/cm-engine-camoufox/ Engine Camoufox + helper Python (python/helper/)
  crates/cm-service/         logic (Manager), mỗi mảng một module
  crates/cm-api/             REST API (axum), mỗi mảng một module
  crates/cm-ui/              UI Vue 3 + Tailwind (assets/), CSS build bằng manager/ui-build.sh
  apps/camoufox-manager/     binary: cửa sổ app, `serve`, CLI — chỉ nối các phần
  gates.sh                   fmt + clippy + build + test
```

## Lưu ý

- Chỉ dùng với profile, tài khoản và hệ thống bạn có quyền quản lý.
- Bảo vệ thư mục dữ liệu vì chứa credential proxy; không expose cổng local qua tunnel/port forwarding khi chưa có xác thực.
- Kiểm tra IP/geo của proxy trước khi Start.
