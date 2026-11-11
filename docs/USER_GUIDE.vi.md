# Hướng dẫn sử dụng

| Mục | Giá trị |
|---|---|
| Trạng thái | Bản xem trước: mô tả tính năng dự kiến; ứng dụng chưa phát hành |
| Phụ trách | Maintainer |
| Cập nhật | 2026-11-09 |
| Bản gốc | [USER_GUIDE.md](USER_GUIDE.md) (luôn cập nhật song song) |

> WalkTunes đang được phát triển. Tài liệu này mô tả cách phiên bản 1.0
> được thiết kế để hoạt động. Các mục ghi *(dự kiến)* hiện chưa có.

---

## 1. WalkTunes làm gì

WalkTunes quản lý nhạc offline của bạn và giữ máy nghe nhạc luôn đồng bộ.
Ứng dụng ghi nhớ trên máy nghe nhạc đang có những gì. Nhờ vậy, cắm máy vào
bất kỳ máy tính nào của bạn, bạn cũng thấy ngay máy có gì, thiếu gì và cái
gì đã thay đổi.

- **Thư viện:** gom mọi thư mục nhạc về một chỗ, tìm kiếm được, tự phát hiện bài trùng và tag lộn xộn.
- **Đồng bộ:** chọn nghệ sĩ, album hoặc playlist, xem trước kế hoạch rồi đồng bộ. Chỉ phần khác biệt được chép.
- **Máy nghe nhạc tự ghi nhớ:** một thư mục ẩn nhỏ trên máy lưu lại những gì đã đồng bộ và từ máy tính nào.
- **Dọn dẹp:** sửa tag và ảnh bìa, chỉnh lại các file tải từ trang video.
- **An toàn ngay từ thiết kế:** file gốc của bạn không bao giờ bị xoá, và file bạn tự chép vào máy nghe nhạc cũng không bao giờ bị gỡ.

## 2. Máy nghe nhạc được hỗ trợ

| Máy | Trạng thái |
|---|---|
| Sony NW-ZX300 | Mục tiêu chính (1.0) |
| Sony NW-A30 / A40 / A50, NW-WM1A / WM1Z | Mục tiêu (1.0), đang kiểm chứng |
| Các máy Sony đời cũ hơn, máy MP3 phổ thông dạng ổ USB | Sau 1.0 *(dự kiến)*; dùng cơ bản qua chế độ "Máy phổ thông" |
| Walkman chạy Android, điện thoại | Không nằm trong mục tiêu (hãy dùng ứng dụng riêng của chúng) |

Máy nghe nhạc của bạn cần hiện lên như một **ổ USB** trên máy tính. Với
ZX300, chỉ cần cắm cáp USB là máy hiện ra như một ổ đĩa (trên macOS, máy
xuất hiện trong Finder).

## 3. Cài đặt *(dự kiến)*
- **macOS:** mở file `.dmg` rồi kéo WalkTunes vào thư mục Applications.
- **Windows:** chạy file `.msi`.
- **Linux:** tải file AppImage, cấp quyền thực thi rồi chạy.

Tính năng chuyển định dạng (đổi những định dạng máy không phát được) dùng
**ffmpeg**, và ffmpeg đã có sẵn trong bộ cài.

## 4. Lần chạy đầu tiên
1. **Chọn ngôn ngữ** (English / Tiếng Việt) và giao diện sáng hoặc tối.
2. **Thêm thư mục nhạc:** *Thư viện → Thêm thư mục*. Thêm bao nhiêu cũng được (ổ trong, ổ ngoài, NAS).
3. WalkTunes **quét** các thư mục. Bạn vẫn duyệt thư viện được trong lúc quét.
4. Tuỳ chọn: **Dịch vụ trực tuyến** (*Cài đặt → Dịch vụ trực tuyến*). Mỗi dịch vụ đều tắt cho đến khi bạn bật: MusicBrainz (tag), Cover Art Archive (ảnh bìa), AcoustID (nhận diện bài không có tag) và LRCLIB (lời bài hát).

## 5. Ghép đôi máy nghe nhạc
1. Cắm máy nghe nhạc bằng cáp USB. Chỉ sau vài giây, WalkTunes **tự nhận ra đó là Sony Walkman** và hiện thông báo *"Đã kết nối Sony NW-ZX300"*. Nếu máy có thẻ microSD, thẻ được hiển thị là một phần của cùng máy.
2. Lần đầu tiên, hãy đặt tên cho máy, ví dụ "ZX300 của tôi", rồi xác nhận. Với những máy WalkTunes chưa nhận diện chắc chắn, ứng dụng sẽ gợi ý dòng máy gần nhất để bạn chọn.
3. Từ đó trở đi, máy được nhận diện tự động trên mọi máy tính có cài WalkTunes.

WalkTunes chỉ *đọc* thông tin nhận dạng mà máy tính vốn đã thấy. Ứng dụng không bao giờ gửi lệnh đặc biệt tới máy nghe nhạc và không bao giờ đụng vào file firmware của máy.

Walkman classic tự mô tả chính nó trong một file nhỏ (`default-capability.xml`) ở thư mục gốc của mỗi bộ nhớ. WalkTunes đọc file này để biết chính xác model, phiên bản firmware và các định dạng máy phát được. Vui lòng đừng xoá file này, cũng như `DevIcon.fil` và `DevLogo.fil`.

Trên NW-ZX300, bộ nhớ trong dùng FAT32 nên không chứa được file từ 4 GB trở lên. WalkTunes sẽ đặt những file như vậy (bài DSD hoặc hi-res dài) vào thẻ microSD, hoặc báo cho bạn biết.

Nếu WalkTunes báo máy đang kết nối ở **chế độ MTP**, hãy đổi chế độ kết nối USB của máy nghe nhạc sang dạng ổ đĩa (mass storage) rồi cắm lại.

WalkTunes tạo một thư mục tên `.walktunes` trên máy nghe nhạc. **Vui lòng đừng
xoá thư mục này.** Nhờ nó mà các máy tính của bạn biết trên máy nghe nhạc
đang có gì.

## 6. Đồng bộ nhạc sang máy nghe nhạc
1. Mở **Thiết bị → máy của bạn → Lựa chọn** và đánh dấu những gì bạn muốn: tất cả, nghệ sĩ, album hoặc playlist. Với mỗi lựa chọn, chọn lưu vào bộ nhớ trong hay thẻ SD.
2. Bấm **Lập kế hoạch đồng bộ**. Bạn sẽ thấy:
   - **Thêm:** bài mới sẽ được chép
   - **Thay thế:** bài đã thay đổi trong thư viện
   - **Gỡ bỏ:** bài bạn đã bỏ chọn (chỉ áp dụng cho file do WalkTunes chép vào)
   - Tổng dung lượng và dung lượng còn trống sau khi đồng bộ
3. Bấm **Đồng bộ**. Bạn vẫn dùng ứng dụng bình thường trong lúc chờ; tiến độ hiện ở mục **Tác vụ**.
4. Khi xong, bấm **Tháo thiết bị** trước khi rút cáp.

Nếu cáp bị tuột giữa chừng, dữ liệu không bị hỏng. Cắm lại máy và bấm
**Tiếp tục**.

**Những gì WalkTunes làm với bản sao (không bao giờ với bản gốc):** thu nhỏ ảnh bìa
để máy hiển thị được, chuyển những định dạng máy không phát được, và đặt
lại tên file cho an toàn với máy.

### Khi bạn cắm máy nghe nhạc vào *(dự kiến)*
Giống iTunes với iPod, WalkTunes sẽ tự phản ứng khi bạn cắm máy nghe nhạc vào.
Một tiến trình nhỏ tên **WalkTunes Helper** chạy nền sau khi bạn đăng nhập:
- máy đã ghép đôi: WalkTunes tự mở thẳng vào trang của máy đó;
- máy mới: hiện thông báo kèm hai nút *Mở* và *Đồng bộ ngay*.

Bạn đổi cách phản ứng trong *Cài đặt → Khi cắm máy nghe nhạc* (Mở WalkTunes /
Chỉ thông báo / Không làm gì), hoặc tắt *Chạy WalkTunes Helper khi đăng nhập*.
Helper không bao giờ chép gì mà không cho bạn xem kế hoạch trước.

## 7. Dùng một máy nghe nhạc với nhiều máy tính
Cắm máy nghe nhạc vào một máy tính khác có cài WalkTunes:
- WalkTunes đọc bản ghi trên máy và hiện ra những bài **có trên máy nghe nhạc nhưng chưa có trên máy tính này**.
- Bấm **Kéo về** để chép các bài đó vào thư mục *Inbox* của máy tính này.
- File bạn tự chép vào máy nghe nhạc sẽ hiện là *Chưa quản lý*. Bạn có thể bấm **Nhận quản lý**, và từ đó WalkTunes sẽ quản lý chúng.

## 8. Dọn dẹp thư viện
Mở mục **Sức khoẻ thư viện** để xem:
- Bài thiếu ảnh bìa, thiếu tag hoặc tag không thống nhất (ví dụ một album bị tách làm hai vì "Album Artist" khác nhau)
- Bài trùng: bản sao y hệt, hoặc cùng một bài ở các chất lượng khác nhau
- Ảnh bìa mà máy nghe nhạc thường không hiển thị được

### Bài tải từ trang video
Với các file tên kiểu `Ca sĩ - Bài hát (Official Video) [HD].mp3`, WalkTunes gợi ý
tên bài và ca sĩ đã được làm gọn, thử nhận diện bài hát qua mạng (nếu bạn đã
bật), và có thể tải ảnh bìa cùng lời bài hát. Bạn duyệt từng gợi ý trước khi
có bất kỳ thay đổi nào. Bạn cũng có thể đánh dấu các file này là *nguồn
lossy* để sau này thay bằng bản chất lượng tốt hơn.

### Kiểm tra chất lượng âm thanh (kiểm tra FLAC và spectrogram) *(dự kiến, M3–M4)*
Mở một bài hoặc một album rồi chọn **Kiểm tra**. Muốn kiểm tra nhiều file một lúc, vào **Sức khoẻ thư viện → Kiểm tra chất lượng**.
- **Tính toàn vẹn:** WalkTunes giải mã toàn bộ file. Với FLAC, ứng dụng còn kiểm tra mã kiểm tra của từng frame và **chữ ký MD5** có sẵn trong file. Kết quả có thể là *OK*, *Thiếu MD5*, hoặc *Lỗi* (file hỏng hoặc bị cắt cụt).
- **Spectrogram:** hình ảnh của âm thanh, giống Spek: thời gian chạy từ trái sang phải, tần số từ dưới lên trên, độ lớn thể hiện bằng màu. Đổi kênh (mid · left · right) bằng các tiêu đề pivot.
- **FLAC này có thật không?** Nếu spectrogram bị cắt ngang thành một đường thẳng ở khoảng 16–20 kHz, file nhiều khả năng được làm từ MP3 hoặc từ bản tải trên trang video. WalkTunes vẽ đường cắt đó và gắn nhãn *Đáng ngờ* hoặc *Có thể từ nguồn lossy*. Ứng dụng cũng phát hiện file "hi-res" giả (upsample) và 24-bit giả (chỉ đệm thêm bit).
- Đây là **gợi ý, không phải phán quyết**. Các bản thu cũ hoặc thu analog có thể trông tương tự. Không có file nào bị xoá hay bị ẩn đi; khi đã yên tâm, bấm **Đánh dấu đã xem**.

## 9. Playlist và lời bài hát *(dự kiến, M5)*
- Tạo playlist ngay trong WalkTunes. Playlist được chép sang máy nghe nhạc dưới dạng file `.m3u8`.
- Lời bài hát (`.lrc`) được chép cạnh từng bài để máy hiển thị được.

## 10. Cam kết an toàn
- WalkTunes **không bao giờ xoá file nhạc gốc của bạn**. Lệnh "Xoá file" chỉ chuyển file vào thùng rác của hệ thống, và chỉ sau khi bạn xác nhận.
- Trên máy nghe nhạc, WalkTunes chỉ gỡ những file **do chính WalkTunes chép vào**, và chỉ khi file đó chưa bị thay đổi.
- Mọi thay đổi đều được ghi lại, và chỉnh sửa tag có thể hoàn tác trong 30 ngày.

## 11. Xử lý sự cố
Lỗi hiển thị kèm mã, ví dụ **WT-SYNC-0002**. Một số lỗi thường gặp:

| Mã | Ý nghĩa | Cách xử lý |
|---|---|---|
| WT-SYNC-0002 | Máy nghe nhạc không đủ dung lượng | Bỏ chọn bớt album, hoặc chuyển sang thẻ SD |
| WT-SYNC-0003 | Máy bị ngắt kết nối khi đang đồng bộ | Cắm lại và bấm Tiếp tục |
| WT-FORGE-0001 | Không tìm thấy ffmpeg | Cài lại WalkTunes hoặc cài ffmpeg |
| WT-DOCK-0004 | Bản ghi trên máy bị hỏng | Bấm **Dựng lại** (WalkTunes đọc lại toàn bộ máy) |
| WT-GATE-0001 | Dịch vụ trực tuyến đang tắt | Bật trong Cài đặt → Dịch vụ trực tuyến |

Nếu cần hỗ trợ, vào *Cài đặt → Xuất thông tin chẩn đoán* và đính kèm file vào
một issue trên GitHub. File này không chứa nhạc và không chứa đường dẫn cá
nhân của bạn.

## 12. Dòng lệnh *(dự kiến)*
```bash
wtunes root add ~/Music
wtunes scan
wtunes device list
wtunes sync plan  --device "ZX300 của tôi"
wtunes sync run   --device "ZX300 của tôi"
wtunes pull       --device "ZX300 của tôi"
wtunes health --json
wtunes inspect ~/Music/Album --json      # tính toàn vẹn + kết luận
wtunes inspect track.flac --png out.png  # lưu ảnh spectrogram
```
