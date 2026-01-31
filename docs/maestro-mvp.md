# Friday x Maestro Test Runner - MVP Dokümantasyonu

## 1) Amaç
Friday içinde Maestro testlerini, kullanıcı CLI görmeden çalıştırmak. Kullanıcı `.yml/.yaml` dosyasını açar, Test Runner panelini açar, platform seçer ve testi çalıştırır. Sağ panelde canlı ekran + log görünür. Chat üzerinden `/test` komutu ile test seçip çalıştırma yapılır.

## 2) Kapsam (MVP)
- Test formatı: Maestro YAML (`*.yml`, `*.yaml`)
- Platformlar: Android, iOS, Web
- Çalıştırma: Test Runner paneli üzerinden manuel
- Chat: `/test` komutu ile test seçimi + çalıştırma
- Önizleme: Emulator/Simulator ekranını periyodik screenshot ile gösterme
- Kullanıcıdan beklenenler: Emulator/Simulator kurulu ve çalışır durumda

## 3) Mimari Özet

### 3.1 Backend (Rust/Tauri)
- Maestro MCP server STDIO üzerinden çalışır.
- Friday backend MCP client olarak STDIO’dan JSON-RPC gönderir/okur.
- Maestro MCP process yönetimi yapılır (spawn/health/kill).

### 3.2 Frontend (React)
- Test Runner paneli:
  - Platform picker (Web/iOS/Android)
  - Run butonu (Rambo)
  - Log alanı
  - Screenshot alanı
- Chat: `/test` ile test listesi + checkbox seçim + çalıştır

## 4) İş Akışı

### 4.1 Panel Üzerinden
1. Kullanıcı `.yml` dosyasını açar.
2. “Test Runner” butonu tıklanır.
3. Platform seçilir (Web/iOS/Android).
4. Run tıklanır.
5. MCP çağrıları:
   - `list_devices`
   - `start_device`
   - `run_flow` veya `run_flow_files`
   - periyodik `take_screenshot`

### 4.2 Chat Üzerinden
1. Kullanıcı `/test` yazar.
2. UI test dosyalarını listeler ve checkbox ile seçtirir.
3. “Çalıştır” tıklanır.
4. Panelde aynı akış başlar.

## 5) Paketleme Stratejisi (MVP)
- Maestro CLI uygulama içine gömülür.
- JRE 17 uygulama içine gömülür (jlink ile minimal).
- Platforma göre ayrı paket (macOS ARM/Intel, Windows, Linux).
- Maestro çalıştırma path’i uygulama resource içinden çözülür.

## 6) Ön Koşullar ve Preflight
- Android SDK + Emulator (Android için)
- Xcode + iOS Simulator (iOS için)
- Web için uygun tarayıcı
- MVP’de: eksikler tespit edilip UI’da uyarı verilir.

## 7) Hata Yönetimi
- MCP bağlantı hatası → “Test Runner bağlantısı koptu”
- Device yok → “Emulator/Simulator açılmalı”
- YAML hatası → `check_flow_syntax` ile gösterilir

## 8) Riskler
- macOS codesign/notarization yapılmazsa binary çalışmayabilir
- STDIO log kirlenmesi JSON-RPC’yi bozabilir
- Screenshot payload büyük olabilir (performans riski)

## 9) Başarı Kriterleri
- YAML aç → Test Runner aç → cihaz seç → test çalıştır → sonuç al
- `/test` → seçim → çalıştır → sonuç al
- Kullanıcı Maestro indirmeden çalıştırabilir

## 10) MVP Sonrası
- Rust tabanlı test runner araştırması
- Multi-device parallel test
- Gelişmiş raporlama (video, timeline, özet)

