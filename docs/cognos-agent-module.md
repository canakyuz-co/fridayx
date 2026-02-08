# Fridex Cognos — Kisisel Otonom AI Asistan Modulu

> **Durum:** Tasarim dokumani (Draft v2 — Cognitive Evolution)
> **Yazar:** Bekircan Akyüz
> **Tarih:** 2025-02-08
> **Bagimliliklar:** fridex-cognos-os-spec.tex, ai-providers.md, maestro-mvp.md

---

# KISIM I: TEMEL MIMARI VE YETENEKLER

---

## 1. Vizyon

Cognos, FridayX ekosistemi icinde calisan bagimsiz bir AI asistan moduludur. Bir chatbot degil, Bilissel Isletim Sistemi (Cognos-OS) ajanlarinin kullaniciya donuk yuzeyi.

JARVIS/FRIDAY vizyonunda: kullanici ile simbiyotik bag kuran, gorev alan, arastiran, kod yazan, test eden, tasarlayan, form dolduran, gerektiginde Companion App uzerinden soran bir kisisel asistan.

**Temel ayrim:** Cognos bir LLM wrapper degil, kendi algi, anlama, karar ve ogrenme pipeline'larina sahip bir **bilissel sistem**dir. LLM'ler bu sistemin bir *bileseni*dir, tamamı degildir.

### 1.1 Temel Ilkeler

| Ilke | Aciklama |
|------|----------|
| **Active Inference** | Komut beklemez, belirsizligi azaltmak icin epistemik bilgi toplar |
| **Privacy-by-Architecture** | Veri mumkun oldugunca local islenir |
| **Deny-Default** | Her arac/erisim izni acikca verilmeli |
| **Audit Trail** | Her eylem loglanir, PII maskelenir |
| **Kullanici Otoritesi** | Kullanici her zaman son onay mercii |
| **Hierarchical Cognition** | Her sorgu en ucuz/hizli katmanda cozulur, gerekirse tirmandirilir |
| **Continuous Learning** | Sistem her etkilesimden ogrenir, zamanla daha iyi olur |

### 1.2 Bu Neden "Just Another Chatbot" Degil

| Chatbot | Cognos |
|---------|--------|
| Komut bekler | Surekli algilar, proaktif onerir |
| Her seyi LLM'e gonderir | Hiyerarsik: cogunluk local'de cozulur |
| Istekler arasi stateless | Surekli dunya modeli, her an guncellenir |
| Tek model | Ensemble: dogru goreve dogru model |
| Ogrenme yok | Surekli adaptasyon |
| Sadece metin algisi | Multi-modal: metin + ses + ekran + sistem |
| Reaktif | Proaktif: ihtiyaci onceden tahmin eder |
| Session bazli hafiza | Kalici, yapisal, cizge tabanli hafiza |

---

## 2. Mimari

Cognos, FridayX uygulamasinin icinde yasayan ama bagimsiz bir moduldur. Mevcut Codex app-server akisi **bozulmaz**.

### 2.1 Katman Diyagrami

```
+─────────────────────────────────────────────────────────────────+
│              KULLANICI (Ses / Metin / Companion App / Davranis)   │
+──────────────────────────┬──────────────────────────────────────+
                           │
+──────────────────────────▼──────────────────────────────────────+
│  PERCEPTION PIPELINE (Always-On)                                 │
│  Ses Akisi │ Ekran Farkindaligi │ Sistem Telemetri │ Dissal      │
+──────────────────────────┬──────────────────────────────────────+
                           │
+──────────────────────────▼──────────────────────────────────────+
│  LOCAL NLP PIPELINE                                              │
│  Intent Classifier │ NER │ Sentiment │ Dialogue State Tracker    │
+──────────────────────────┬──────────────────────────────────────+
                           │
+──────────────────────────▼──────────────────────────────────────+
│                     UI KATMANI                                   │
│  Chat Modu │ Companion Overlay │ Background + Notification       │
+──────────────────────────┬──────────────────────────────────────+
                           │ Tauri IPC
+──────────────────────────▼──────────────────────────────────────+
│                   ORCHESTRATOR (Rust)                             │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌───────────────────────┐  │
│  │  Task Queue   │  │ Policy Engine│  │ World Model           │  │
│  │  (tokio)      │  │ (deny-default│  │ (KG + Temporal +      │  │
│  │               │  │  + RBAC)     │  │  Predictive State)    │  │
│  └──────┬───────┘  └──────┬───────┘  └───────────────────────┘  │
│         │                 │                                       │
│  ┌──────▼─────────────────▼──────────────────────────────────┐   │
│  │         HIERARCHICAL DECISION ENGINE                       │   │
│  │  L0: Pattern │ L1: Local SM │ L2: Local LM │ L3: Cloud    │   │
│  └──────┬────────────────────────────────────────────────────┘   │
│         │                                                         │
│  ┌──────▼────────────────────────────────────────────────────┐   │
│  │              PROVIDER ROUTER                               │   │
│  │  Claude API/CLI │ Gemini API/CLI │ Ollama │ Custom         │   │
│  └──────┬────────────────────────────────────────────────────┘   │
│         │                                                         │
│  ┌──────▼────────────────────────────────────────────────────┐   │
│  │                    TOOL LAYER                              │   │
│  │  code │ test │ design │ web │ shell │ git │ notify │       │   │
│  │  browser │ file │ calendar                                 │   │
│  └────────────────────────────────────────────────────────────┘   │
│         │                                                         │
│  ┌──────▼────────────────────────────────────────────────────┐   │
│  │              CONTINUOUS LEARNING PIPELINE                  │   │
│  │  Preference │ Skill Acquisition │ Environmental Feedback   │   │
│  └────────────────────────────────────────────────────────────┘   │
+─────────────────────────────────────────────────────────────────+
```

### 2.2 Orchestrator (Rust, Tauri Backend)

Tum karar ve koordinasyonun merkezi. Mevcut Tauri backend'ine yeni bir modul olarak eklenir.

**Sorumluluklar:**
- Gorev kuyrugu yonetimi (task queue, tokio async)
- Arac cagri izin motoru (policy engine, deny-default)
- Provider routing: hangi LLM'e gidecek, maliyet/kalite optimizasyonu
- World model yonetimi: algi → guncelleme → tahmin dongusu
- Event hub: tum modullerle iletisim (Tauri IPC)

### 2.3 Provider Katmani ve Model Secim Teorisi

Mevcut `ai-providers.md` dokumandaki sistem **aynen korunur**. Cognos bu katmani kullanir ama degistirmez.

| Provider | Protokol | Kullanim Alani |
|----------|----------|----------------|
| Claude Opus/Sonnet | API / CLI | Kod yazma, karmasik reasoning |
| Gemini | API / CLI | Uzun dokuman analizi (1M context) |
| Ollama (local) | OpenAI-compatible | Offline, gizli veri, hizli sorgu |
| OpenAI | API | OpenAI-compatible endpoint'ler |
| Custom | API / CLI / ACP | Kullanici tanimli |

#### 2.3.1 Multi-Armed Bandit ile Model Secimi

Model secimi bir **Contextual Multi-Armed Bandit** problemi olarak modellenir. Her model bir "kol", her gorev tipi bir "baglam"dir.

**Tanim:** $K$ adet model (kol), $d$ boyutlu baglam vektoru $\mathbf{x}_t \in \mathbb{R}^d$.

Baglam vektoru:
$$\mathbf{x}_t = [\text{task\_type}, \text{complexity}, \text{privacy\_level}, \text{urgency}, \text{context\_length}]$$

Her model $k$ icin beklenen odul (LinUCB algoritmasiyla):

$$\hat{r}_{t,k} = \mathbf{x}_t^\top \hat{\boldsymbol{\theta}}_k + \alpha \sqrt{\mathbf{x}_t^\top \mathbf{A}_k^{-1} \mathbf{x}_t}$$

Burada:
- $\hat{\boldsymbol{\theta}}_k$: model $k$ icin ogrenilmis parametre vektoru
- $\mathbf{A}_k = \mathbf{I}_d + \sum_{\tau} \mathbf{x}_\tau \mathbf{x}_\tau^\top$: tasarim matrisi
- $\alpha$: exploration-exploitation dengesi parametresi

**Secim kurali:**
$$k^* = \arg\max_k \hat{r}_{t,k}$$

**Odul fonksiyonu:**
$$r_t = w_1 \cdot Q(k, \text{task}) + w_2 \cdot \frac{1}{L(k)} - w_3 \cdot C(k) + w_4 \cdot P(k)$$

| Sembol | Anlam | Olcum |
|--------|-------|-------|
| $Q(k, \text{task})$ | Kalite skoru | Gorev basari orani (0-1) |
| $L(k)$ | Latency | Yanit suresi (saniye) |
| $C(k)$ | Maliyet | Dolar / 1K token |
| $P(k)$ | Privacy bonusu | Local = 1.0, Cloud = 0.0 |

**Parametre guncelleme (her etkilesim sonrasi):**
$$\mathbf{A}_k \leftarrow \mathbf{A}_k + \mathbf{x}_t \mathbf{x}_t^\top$$
$$\mathbf{b}_k \leftarrow \mathbf{b}_k + r_t \mathbf{x}_t$$
$$\hat{\boldsymbol{\theta}}_k \leftarrow \mathbf{A}_k^{-1} \mathbf{b}_k$$

> **Uretim basitlestirmesi:** Ilk fazlarda kural tabanli routing yeterlidir. Bandit sistemi yeterli veri toplandiktan sonra (N > 100 etkilesim) devreye girer.

### 2.4 Tool Layer

Her arac bir MCP server veya Tauri command olarak calisir. Her biri bagimsiz, izole, izin kontrollu.

| Arac | Tanim | Izin Seviyesi |
|------|-------|---------------|
| `code` | Dosya okuma/yazma/duzenleme (workspace ici) | Okuma: Seviye 1, Yazma: Seviye 2 |
| `test` | Maestro MCP uzerinden test calistirma | Seviye 2 |
| `design` | Pencil MCP uzerinden .pen okuma/yazma | Seviye 2 |
| `web` | Arama, sayfa okuma, link toplama | Seviye 1 |
| `shell` | Terminal komutu calistirma | Seviye 3 |
| `git` | Branch, commit, PR islemleri | Commit: Seviye 2, Push/PR: Seviye 3 |
| `notify` | Companion App / macOS bildirim | Seviye 3 |
| `browser` | URL acma, form doldurma | Seviye 3 |
| `file` | Dosya sistemi (workspace disi) | Seviye 3 |
| `calendar` | Takvim okuma/yazma | Seviye 3 (ileride) |

### 2.5 Memory Layer ve Cizge Tabanli Hafiza Matematigi

#### 2.5.1 Hafiza Uzayi Tanimi

Cognos-OS spec'teki cizge tabanli hafiza:

$$G = (V, E, \mathcal{A}, \mathcal{R})$$

- $V$: Varliklar kumesi — User, Project, File, Task, Contact, Preference, Document
- $E \subseteq V \times V$: Varlik iliskileri
- $\mathcal{A}: V \rightarrow \mathbb{R}^d$: Dugum oznitelik vektorleri (embeddings)
- $\mathcal{R}$: Iliski tipleri — OWNS, DEPENDS\_ON, PREFERS, WORKED\_ON, KNOWS\_ABOUT

#### 2.5.2 TransE: Bilgi Cizgesi Gomme (Knowledge Graph Embedding)

Iliskiler vektör uzayinda modellenir. Bir $(h, r, t)$ uclusu (head, relation, tail) icin:

$$\mathbf{h} + \mathbf{r} \approx \mathbf{t}$$

**Kayip fonksiyonu (margin-based):**

$$\mathcal{L} = \sum_{(h,r,t) \in S} \sum_{(h',r,t') \in S'} \max(0, \gamma + d(\mathbf{h}+\mathbf{r}, \mathbf{t}) - d(\mathbf{h'}+\mathbf{r}, \mathbf{t'}))$$

Burada:
- $S$: pozitif (gercek) ucluler kumesi
- $S'$: negatif (bozulmus) ucluler kumesi
- $\gamma$: margin hiperparametresi
- $d(\cdot, \cdot)$: L1 veya L2 mesafe

**Ornek:**
```
(User, WORKS_ON, ProjectFridex) → e_user + e_works_on ≈ e_fridex
(ProjectFridex, CONTAINS, FileAuthTs) → e_fridex + e_contains ≈ e_auth_ts
```

#### 2.5.3 GNN ile Cizge Uzerinde Mesaj Gecisi (Message Passing)

Dugum temsilleri komsuluk bilgisiyle zenginlestirilir:

$$\mathbf{h}_v^{(l+1)} = \sigma\left(\mathbf{W}^{(l)} \cdot \text{AGG}\left(\{\mathbf{h}_u^{(l)} : u \in \mathcal{N}(v)\}\right) + \mathbf{b}^{(l)}\right)$$

Burada:
- $\mathbf{h}_v^{(l)}$: dugum $v$'nin $l$. katmandaki temsili
- $\mathcal{N}(v)$: dugum $v$'nin komsulari
- $\text{AGG}$: toplama fonksiyonu (mean, sum, veya attention-based)
- $\sigma$: aktivasyon fonksiyonu (ReLU, GELU)

**Attention-based toplama (GAT):**

$$\alpha_{vu} = \frac{\exp(\text{LeakyReLU}(\mathbf{a}^\top [\mathbf{W}\mathbf{h}_v \| \mathbf{W}\mathbf{h}_u]))}{\sum_{k \in \mathcal{N}(v)} \exp(\text{LeakyReLU}(\mathbf{a}^\top [\mathbf{W}\mathbf{h}_v \| \mathbf{W}\mathbf{h}_k]))}$$

$$\mathbf{h}_v^{(l+1)} = \sigma\left(\sum_{u \in \mathcal{N}(v)} \alpha_{vu} \mathbf{W} \mathbf{h}_u^{(l)}\right)$$

> Bu, "User → Project → File" zincirinde baglam yayilimini saglar. Kullanici bir projeyle calisirken, o projenin dosyalari otomatik olarak yuksek relevance alir.

#### 2.5.4 Hibrit Erisim Skorlamasi

Bir sorgu $q$ ile dugum $v$ arasindaki alaka duzeyi:

$$S(q, v) = \alpha \cdot \frac{\mathbf{e}_q \cdot \mathbf{e}_v}{\|\mathbf{e}_q\| \|\mathbf{e}_v\|} + (1-\alpha) \cdot e^{-\lambda \cdot d(v, v_{\text{focus}})}$$

- $\mathbf{e}_q, \mathbf{e}_v$: gomme vektorleri
- $d(\cdot, \cdot)$: en kisa yol uzakligi (Dijkstra veya BFS)
- $\lambda$: sonumleme katsayisi (uzak dugumler dusuk skor alir)
- $\alpha$: semantik vs yapisal denge ($\alpha = 0.6$ baslangic degeri)

**Temporal Decay (zamansal sonumleme):**

$$S_{\text{temporal}}(q, v) = S(q, v) \cdot e^{-\mu \cdot (t_{\text{now}} - t_{\text{last\_access}}(v))}$$

Son erisilen dugumler daha yuksek skor alir. $\mu$ kullanici davranisina gore adapte edilir.

**Depolama:**
- Kisa sureli: son N etkilesim (ring buffer, RAM)
- Uzun sureli: SQLite (yapisal veri) + local vector store (embedding)
- Kullanici profili: tercihler, sik yapilan islemler, red/onay kaliplari

### 2.6 Voice Layer

| Bilesen | Teknoloji | Notlar |
|---------|-----------|--------|
| STT | Whisper.cpp (local) | Privacy-first, offline calisir |
| TTS | macOS AVSpeechSynthesizer | Sistem sesi, sifir maliyet |
| TTS (alternatif) | ElevenLabs API | Dogal ses, bulut, maliyet var |
| Wake word | Opsiyonel ("Hey Friday") | Ileride |
| Varsayilan mod | Push-to-talk | Kullanici basili tutarak konusur |

Ses katmaninin matematigi Bolum 14.1'de detaylandirilmistir.

---

## 3. Yetenek Alanlari

### 3.1 Kod Yazma ve Duzenleme

Kullanici dogal dille anlatir, Cognos kodu yazar, test eder, raporlar.

**Operasyonel Sozlesme (mevcut Cognos-OS spec'ten):**
- Algoritmik analiz zorunlu: her kritik fonksiyon icin time/space (Big-O)
- Minimal diff: sadece gerekli satirlar degisir
- Yasaklar: brute-force, gereksiz nested loop, global state mutasyonu, asiri test
- Guvenlik: PII maskeleme, log kisintisi, secret yonetimi
- Dogrulama: lint + typecheck + ilgili testler; backend degistiyse `cargo check`
- Self-review: edge-case + performans + guvenlik + karmasiklik

**Workflow (hiyerarsik karar ile):**

```
1. Kullanici istegi
   ↓
2. Local intent classifier (Seviye 1, <10ms)
   → intent = code_write | code_fix | code_explain | ...
   → confidence < threshold? → Seviye 3'e tirmandir
   ↓
3. NER ile varlik cikarimi
   → dosya adi, fonksiyon adi, degisken adi, hata mesaji
   ↓
4. Knowledge graph'tan baglam topla
   → ilgili dosyalar, bagimliliklar, test dosyalari
   ↓
5. Uygun model sec (Bandit veya kural tabanli)
   → basit fix? → Ollama local
   → karmasik mimari? → Claude Opus
   ↓
6. Plan oner → kullanici onaylari
   ↓
7. Kod yaz → lint + typecheck → test yaz → test calistir
   ↓
8. Sonuc raporla + ogrenme pipeline'a geri bildirim
```

### 3.2 Test Yazma ve Calistirma

Maestro MCP entegrasyonu ile platform bazli test.

**Desteklenen Test Tipleri:**

| Tip | Arac | Format |
|-----|------|--------|
| Unit | Vitest | TypeScript (*.test.ts) |
| E2E | Maestro | YAML flow (*.yml) |
| Integration | Vitest + API | TypeScript |
| Visual Regression | Maestro screenshot | PNG kiyaslama |

**Maestro MCP Akisi:**
```
list_devices → start_device → run_flow → take_screenshot → sonuc analizi
```

#### 3.2.1 Gorsel Regresyon Testi — Perceptual Hash ile Karsilastirma

Screenshot karsilastirmasi icin pixel-bazli diff yerine **perceptual hashing**:

$$h(I) = \text{sign}(\text{DCT}(\text{resize}(I, 32 \times 32)) - \text{median}(\text{DCT}))$$

Iki goruntu arasindaki fark:

$$d(I_1, I_2) = \frac{\text{hamming}(h(I_1), h(I_2))}{|h|}$$

- $d < 0.1$: ayni goruntu (test gecti)
- $0.1 \leq d < 0.3$: kucuk fark (inceleme gerekir)
- $d \geq 0.3$: onemli fark (test kaldi)

### 3.3 Tasarim (Pencil Entegrasyonu)

.pen dosyalarini okur, analiz eder, tasarlar, dogrular.

**Kullanilacak MCP Araclari:**

| Arac | Amac |
|------|------|
| `batch_get` | Mevcut komponentleri kesfet |
| `get_guidelines` | Tasarim kurallarini oku |
| `get_style_guide` | Stil rehberi al |
| `batch_design` | Tasarim uygula (I/U/D/R/C/M/G) |
| `get_screenshot` | Gorsel dogrulama |
| `snapshot_layout` | Layout sorunlarini tespit et |

### 3.4 Arastirma ve Bilgi Toplama

Web aramasi, dokuman analizi, kaynak dogrulama.

#### 3.4.1 Kaynak Guvenilirlik Skorlamasi

Birden fazla kaynaktan gelen bilgi capraz dogrulanir:

$$\text{Reliability}(c) = \frac{1}{N} \sum_{i=1}^{N} \mathbb{1}[\text{source}_i \text{ confirms } c] \cdot w(\text{source}_i)$$

Kaynak agirligi:
$$w(s) = \text{domain\_authority}(s) \cdot \text{recency}(s) \cdot \text{relevance}(s)$$

- $\text{Reliability} > 0.8$: guvenilebilir, kullaniciya sun
- $0.5 < \text{Reliability} \leq 0.8$: "su kaynaklara gore..." diye belirt
- $\text{Reliability} \leq 0.5$: "dogrulanamadi" uyarisi

### 3.5 Uzun Sureli Gorev Yonetimi

Gorevin boyutundan bagimsiz, gun boyu calisan bir asistan.

**Gorev Yasam Dongusu:**
```
                    ┌──────────┐
                    │ RECEIVED │
                    └────┬─────┘
                         ↓
                  ┌──────────────┐
                  │ DECOMPOSE    │ Buyuk gorevi alt gorevlere bol
                  └──────┬───────┘
                         ↓
                  ┌──────────────┐
              ┌───│ EXECUTE      │───┐
              │   └──────────────┘   │
              ↓                      ↓
       ┌────────────┐        ┌─────────────┐
       │ CHECKPOINT │        │ BLOCKED     │
       │ (devam et) │        │ (soru sor)  │
       └─────┬──────┘        └──────┬──────┘
             │                      ↓
             │               ┌─────────────┐
             │               │ NOTIFY USER │ (Companion App/UI)
             │               └──────┬──────┘
             │                      ↓
             │               ┌─────────────┐
             │               │ WAIT REPLY  │
             │               └──────┬──────┘
             │                      │
             ←──────────────────────┘
             ↓
       ┌──────────────┐
       │ COMPLETE     │ Ozet rapor
       └──────────────┘
```

#### 3.5.1 Gorev Dekompozisyonu — Hierarchical Task Network (HTN)

Buyuk gorevler otomatik olarak alt gorevlere ayristirilir:

$$\text{Task}(T) = \{t_1, t_2, \ldots, t_n\}$$

Her alt gorev icin:
$$\text{effort}(t_i) = f(\text{complexity}(t_i), \text{dependencies}(t_i), \text{risk}(t_i))$$

**Siralama (topological sort + oncelik):**
$$\text{order} = \text{TopSort}(\text{DAG}(T)) \text{ weighted by } \text{priority}(t_i) \cdot \text{urgency}(t_i)$$

Bagimlilik cizgesi bir DAG (Directed Acyclic Graph) olarak modellenir. Dongusel bagimlilik tespit edilirse kullaniciya uyari verilir.

**Checkpoint Mekanizmasi:**
- Her alt gorev tamamlandiginda durum kaydedilir
- Uygulama kapatilsa bile kaldigi yerden devam eder
- Checkpoint: SQLite'a gorev durumu + context snapshot

### 3.6 Bildirim ve Iletisim

Cognos, kullaniciya ulasmanin birden fazla kanalini kullanir.

**Kanal Onceligi (aciliyet sirasi):**

| Seviye | Kanal | Ornek |
|--------|-------|-------|
| Low | UI log / notification center | "Gorev tamamlandi" |
| Medium | macOS notification | "Test basarisiz, mudahale gerekebilir" |
| High | Companion App push | "2 secenekten birini secmen lazim, devam edemiyorum" |
| Critical | Companion App + macOS alert | "Guvenlik sorunu tespit edildi" |

**Companion App Entegrasyonu (bkz. Bolum 15):**
- FCM + Supabase Realtime ile cift yonlu iletisim
- Mesaj tipleri:
  - Soru: "X konusunda su 2 secenek var, hangisi?"
  - Durum: "Y gorevi tamamlandi, sonuc: ..."
  - Onay: "Z islemini yapmami istiyor musun? [Evet/Hayir]"
  - Uyari: "Hata olustu, mudahale gerekiyor"
- Kullanici Companion App'ten cevap verir → Cognos devam eder

### 3.7 Form Doldurma ve Belge Islemleri

**Guvenlik Kurali:** ASLA otomatik submit ETMEZ. Kullanici her zaman son kontrol yapar.

**Workflow (ornek: vize basvurusu):**
```
1. Girdi: ulke + vize tipi
   ↓
2. Web'den gereksinimler arastir
   → Konsolosluk sitesi, resmi kaynaklar
   → Capraz dogrulama (Reliability skoru)
   ↓
3. Gerekli belge kontrol listesi olustur
   ↓
4. Hafizadan (Knowledge Graph) bilinen bilgileri doldur
   → Ad, soyad, dogum tarihi, pasaport no...
   → Bilinmeyen alanlari kullaniciya sor
   ↓
5. Form alanlarini doldur (browser automation)
   ↓
6. Kullaniciya goster → "Kontrol et, duzeltmemi istedigin yer var mi?"
   ↓
7. Kullanici onaylar → AMA SUBMIT ETMEZ, kullanici kendisi yapar
```

---

## 4. Karar Mekanizmasi: Matematiksel Temeller

### 4.1 Variasyonel Serbest Enerji (VFE) — Active Inference

Cognos-OS spec'teki temel prensip. Sistem, duyusal girdi ile icsel dunya modeli arasindaki "surprizi" minimize eder:

$$\mathcal{F} = \mathrm{D}_{KL}[Q(s) \| P(s, o)] = \underbrace{\mathbb{E}_Q[-\ln P(o|s)]}_{\text{Reconstruction Error}} + \underbrace{\mathrm{D}_{KL}[Q(s) \| P(s)]}_{\text{Complexity}}$$

Burada:
- $Q(s)$: sistemin inanci (approximate posterior)
- $P(s, o)$: gercek dunya modeli (generative model)
- $o$: gozlem (kullanici girdisi, dosya degisikligi, sistem durumu)
- $s$: gizli durum (kullanicinin niyeti, projenin durumu)

**Pratik anlam:** Sistem surekli olarak "gerceklikle ne kadar uyumluyum?" sorusunu sorar. Uyumsuzluk yuksekse (surpriz), ya inancini gunceller (perception) ya da dunyayi degistirir (action).

### 4.2 Beklenen Serbest Enerji (EFE) — Karar

Bir politika $\pi$ secilirken:

$$G(\pi) = \sum_{\tau=t}^{T} G(\pi, \tau)$$

$$G(\pi, \tau) \approx \underbrace{-\mathbb{E}_{Q}[\ln P(o_\tau | C)]}_{\text{Pragmatic Value (Goal)}} - \underbrace{\mathbb{E}_{Q}\left[\mathrm{D}_{KL}[Q(s_\tau | o_\tau, \pi) \| Q(s_\tau | \pi)]\right]}_{\text{Epistemic Value (Curiosity)}}$$

- **Pragmatic value:** Hedefe ne kadar yaklasiyoruz?
- **Epistemic value:** Bu eylem ne kadar bilgi kazandirir? (merak mekanizmasi)

**Ornek:** Kullanici "bu bug'i fix'le" dediginde:
- Eylem A: Dogrudan kodu degistir → Pragmatic value yuksek, epistemic dusuk
- Eylem B: Once hata loglarini oku → Pragmatic dusuk ama epistemic yuksek
- Sistem B'yi secer cunku toplam $G(\pi)$ dusurur (belirsizligi azaltir)

### 4.3 Inanc Guncelleme (POMDP)

Durum tamamen gozlemlenemez (Partially Observable). Inanc durumu:

$$B_t(s) = P(S_t = s \mid o_{1:t}, a_{1:t-1})$$

**Bayesyen guncelleme (her yeni gozlemde):**

$$B_{t+1}(s') = \eta \cdot \underbrace{P(o_{t+1} | s')}_{\text{Observation Model}} \cdot \sum_{s} \underbrace{P(s' | s, a_t)}_{\text{Transition Model}} \cdot B_t(s)$$

- $\eta$: normalizasyon sabiti ($\sum_{s'} B_{t+1}(s') = 1$)
- $P(o_{t+1} | s')$: Bu durumda bu gozlemi gorme olasiligi
- $P(s' | s, a_t)$: Eylem $a_t$ sonrasi durumun degisme olasiligi

**Pratik uygulama:**

| Gozlem $o_t$ | Durum $s$ | Guncelleme |
|---|---|---|
| Kullanici auth.ts acti | $P(\text{auth\_work}) \uparrow$ | Proje baglami guncelle |
| Hizli yazim, sik silme | $P(\text{frustrated}) \uparrow$ | Yanit tonunu ayarla |
| 3 saat ayni dosyada | $P(\text{deep\_focus}) \uparrow$ | Rahatsiz etme |
| "neden calismıyor" yazdi | $P(\text{debugging}) \uparrow$ | Debug araclari oner |

### 4.4 Bayesyen Niyet Cozumleme

$$P(\text{Intent} \mid \text{Prompt}, \text{Context}) = \frac{P(\text{Prompt} \mid \text{Intent}) \cdot P(\text{Intent} \mid \text{Context})}{P(\text{Prompt})}$$

**Prior $P(\text{Intent} \mid \text{Context})$:** World model'den gelir.
- Saat 09:00, auth.ts acik, dunku bug issue var → $P(\text{bug\_fix})$ yuksek prior
- Cuma 17:00, refactor branch'inde → $P(\text{refactor})$ yuksek prior

**Likelihood $P(\text{Prompt} \mid \text{Intent})$:** Intent classifier'dan gelir.
- "duzelt" → $P(\text{bug\_fix}) = 0.7$, $P(\text{refactor}) = 0.2$, $P(\text{typo}) = 0.1$

**Posterior (guncellenmis inanc):**
- Hepsini carpar, normalize eder → en yuksek posterior olan niyet secilir

### 4.5 Utility Fonksiyonu (Uretim Basitlestirmesi)

EFE'nin pratik yaklasimlasi:

$$U(a) = w_1 \cdot \text{Sim}(\text{Result}(a), \text{Goal}) + w_2 \cdot \text{InfoGain}(a) - w_3 \cdot \text{Cost}(a) - w_4 \cdot \text{Risk}(a)$$

Agirliklar kullanici davranisina gore adapte edilir (Bolum 16'da aciklanan ogrenme dongusu ile).

---

## 5. Guvenlik ve Izin Modeli

### 5.1 Izin Seviyeleri

**SEVIYE 1 — Otomatik (izin gereksiz):**
- Dosya okuma (workspace ici)
- Web aramasi
- Hafiza okuma
- Kod analizi / lint / typecheck

**SEVIYE 2 — Bildirimli (kullanici bilgilendirilir):**
- Dosya yazma/duzenleme (workspace ici)
- Test calistirma
- Git commit (push haric)
- Tasarim duzenleme (.pen)

**SEVIYE 3 — Onay gerekli (kullanici acikca onaylar):**
- Shell komutu calistirma
- Git push / PR olusturma
- Companion App mesaj gonderme
- Browser'da URL acma
- Workspace disi dosya islemi
- Form doldurma
- API key / credential kullanimi

**SEVIYE 4 — Yasak (asla yapamaz):**
- Credential'lari loglamak
- Kullanici adina submit/odeme yapmak
- Workspace disi dosya silmek (onay ile bile)
- Baska kullanicilarin verilerine erismek
- PII'yi disariya gondermek

### 5.2 Anomali Tespiti — Izin Ihlali Erken Uyari

Normal kullanim kaliplarini ogrenip sapmalari tespit et:

$$z_t = \frac{x_t - \mu_{\text{window}}}{\sigma_{\text{window}}}$$

$|z_t| > 3$ ise anomali alarmi:
- Normalden fazla shell komutu calistirma
- Alisik olunmayan saatte yuksek izin seviyesi isteme
- Workspace disi dosya erisim paterni

### 5.3 Audit Trail

Her eylem loglanir:

```json
{
  "timestamp": "2025-02-08T14:32:00Z",
  "action": "file_write",
  "tool": "code",
  "target": "src/components/App.tsx",
  "permission_level": 2,
  "user_notified": true,
  "result": "success",
  "pii_detected": false,
  "model_used": "ollama:codellama",
  "decision_level": 1,
  "confidence": 0.94
}
```

---

## 6. Kullanici Deneyimi (UI Modlari)

### 6.1 Chat Modu (Ana Ekran)

Mevcut FridayX chat arayuzu uzerinden calisir. Ek olarak:
- Ses giris butonu (push-to-talk)
- Gorev ilerleme bari (uzun gorevlerde)
- Zengin yanitlar: kod bloklari, linkler, kontrol listeleri, diff'ler
- Model badge (hangi LLM kullanildigini gosterir)
- Karar seviyesi gostergesi (L0/L1/L2/L3)

### 6.2 Companion Modu (Overlay)

```
┌──────────────────────────────────┐
│  Baska uygulama (browser vs.)    │
│                                  │
│                        ┌───────┐ │
│                        │ 🟢    │ │  ← Kucuk floating widget
│                        │Friday │ │     Durumu gosterir
│                        └───────┘ │     Tikla → mini chat acar
│                                  │
└──────────────────────────────────┘
```

- Ekranin kosesinde kucuk widget
- Durumu gosterir: idle / working / waiting for input / learning
- Tiklayinca genisler → mini chat penceresi
- Ses ile etkilesim (push-to-talk)
- Kullanici baska uygulama kullanirken bile aktif
- macOS'ta NSPanel veya overlay window olarak

### 6.3 Background Modu (Gorunmez)

- Arka planda gorev calistirir
- Gorunur UI yok
- Sadece bildirim ile iletisim:
  - macOS notification (medium oncelik)
  - Companion App push (high oncelik)
- Gorev tamamlaninca veya soru olunca bildirir
- Menu bar'da kucuk ikon ile durum gosterir

---

## 7. Teknik Stack

| Katman | Teknoloji | Notlar |
|--------|-----------|--------|
| Orchestrator | Rust (Tauri backend) | Mevcut altyapiya modul olarak eklenir |
| Task Queue | Rust tokio async tasks | Uzun sureli gorevler icin |
| Local ML Inference | candle (Rust, HuggingFace) | Metal acceleration, Tauri-native |
| Apple Silicon ML | Core ML / MLX | Neural Engine uzerinde <10ms inference |
| Memory (yapisal) | SQLite | Kullanici profili, gorev gecmisi, tercihler |
| Memory (cizge) | oxigraph (Rust, embedded) | SPARQL destekli knowledge graph |
| Memory (vektorel) | Local vector store + HNSW | Embedding tabanli benzerlik aramasi |
| LLM Routing | Mevcut provider katmani | api / cli / acp protokolleri |
| Tools | MCP protocol | Maestro, Pencil, ozel araclar |
| STT | Whisper.cpp / mlx-whisper | Privacy-first, offline |
| TTS | macOS AVSpeechSynthesizer | Sifir maliyet, sistem sesi |
| TTS (alternatif) | ElevenLabs API | Dogal ses, bulut |
| VAD | silero-vad (ONNX, <5MB) | Konusma/sessizlik tespiti |
| Intent Classifier | DistilBERT → Core ML | <100MB, <10ms |
| NER | spaCy small veya custom CRF | Varlik cikarimi |
| Notifications | Companion App (FCM + Supabase) | Asenkron kullanici iletisimi |
| Browser Automation | Playwright veya AppleScript | Form doldurma, URL acma |
| Frontend | React (mevcut FridayX) | Yeni komponentler mevcut sisteme eklenir |

---

## 8. Faz Plani

### Faz 0 — Temel Iskelet (LLM Wrapper)
- [ ] Orchestrator modulu (Rust, Tauri icinde)
- [ ] Provider routing (mevcut ai-providers sistemi uzerinden)
- [ ] Basit chat entegrasyonu (Cognos persona ile)
- [ ] Izin motoru iskeleti (deny-default)

### Faz 1 — Arac Katmani (Kod + Test + Git)
- [ ] `code` tool: workspace dosya okuma/yazma
- [ ] `test` tool: Vitest calistirma
- [ ] `test` tool: Maestro MCP entegrasyonu
- [ ] `git` tool: branch, commit, diff
- [ ] `shell` tool: terminal komutlari (izinli)
- [ ] Izin motoru tam calismasi (Seviye 1-4)

### Faz 2 — Arastirma + Hafiza
- [ ] `web` tool: arama, sayfa okuma
- [ ] `browser` tool: link acma
- [ ] Hafiza katmani: SQLite + vector store
- [ ] Kullanici profili: tercihler, red/onay kaliplari
- [ ] Kaynak dogrulama mekanizmasi (Reliability skoru)

### Faz 3 — Tasarim Entegrasyonu
- [ ] `design` tool: Pencil MCP uzerinden .pen okuma
- [ ] `design` tool: Pencil MCP uzerinden .pen yazma
- [ ] Tasarim dogrulama (screenshot + perceptual hash)
- [ ] Design system kesfetme ve uyum kontrolu

### Faz 4 — Local NLP Pipeline (LLM'den Bagimsizlasma Baslangici)
- [ ] Intent classifier egitimi (DistilBERT, Core ML export)
- [ ] NER modeli (dosya, tarih, kisi, proje cikarimi)
- [ ] Hiyerarsik karar motoru (L0-L3)
- [ ] Kural tabanli model routing → bandit gecisi icin veri toplama

### Faz 5 — Ses + Algi
- [ ] Whisper.cpp entegrasyonu (local STT)
- [ ] silero-vad entegrasyonu (konusma tespiti)
- [ ] macOS TTS entegrasyonu
- [ ] Push-to-talk UI
- [ ] Companion overlay modu (floating widget)
- [ ] Ekran farkindaligi (macOS Accessibility API)

### Faz 6 — Bildirim + Uzun Gorev
- [ ] Companion App entegrasyonu (FCM + Supabase Realtime)
- [ ] Bildirim oncelik sistemi (low/medium/high/critical)
- [ ] Uzun gorev yonetimi (HTN dekompozisyon + checkpoint)
- [ ] Background modu (menu bar ikon)
- [ ] Gorev devam ettirme (uygulama yeniden baslatilsa bile)

### Faz 7 — Form + Belge
- [ ] Browser automation (Playwright veya AppleScript)
- [ ] Form alan tespiti ve doldurma
- [ ] Belge arastirma ve kontrol listesi olusturma
- [ ] Kullanici onay akisi (asla otomatik submit)

### Faz 8 — Knowledge Graph + Dunya Modeli
- [ ] oxigraph entegrasyonu (Rust embedded graph DB)
- [ ] TransE ile bilgi cizgesi gomme
- [ ] GNN mesaj gecisi ile baglam yayilimi
- [ ] Temporal model (gunluk/haftalik kullanici kaliplari)
- [ ] Predictive state (sonraki eylem tahmini)

### Faz 9 — Surekli Ogrenme
- [ ] Tercih ogrenme (kullanici duzenlemelerinden)
- [ ] LoRA fine-tuning pipeline (local, privacy-first)
- [ ] Knowledge distillation (Cloud LLM → Local model)
- [ ] Multi-Armed Bandit model secimi (yeterli veri sonrasi)
- [ ] Anomali tespiti (izin kalip analizi)

### Faz 10 — Proaktif Zeka
- [ ] Proaktif oneriler (dunya modeli + tahmine dayali)
- [ ] Duygu/ton analizi (ses + metin)
- [ ] Baglam duyarli bildirim zamanlama
- [ ] Wake word (opsiyonel)
- [ ] Tam otonom ajan modu (kullanici tarafindan yetkilendirilmis gorevlerde)

---

## 9. Mevcut Sistemle Iliski

### 9.1 Codex App-Server

**Degismez.** Cognos, Codex akisini kullanmaz ve bozmaz. Ikisi paralel calisir:
- Codex: workspace bazli ajan orkestrasyon (mevcut JSON-RPC akisi)
- Cognos: kisisel asistan (ayri modul, ayri gorev kuyrugu)

### 9.2 AI Providers

Cognos, `ai-providers.md`'deki provider katmanini **kullanir ama degistirmez**:
- `providerId:modelName` formati korunur
- API/CLI/ACP protokolleri aynen kullanilir
- Ek olarak: hiyerarsik karar motoru ve bandit model secimi eklenir (Cognos'a ozel)

### 9.3 Maestro Test Runner

`maestro-mvp.md`'deki MCP entegrasyonu **aynen kullanilir**:
- `list_devices`, `start_device`, `run_flow`, `take_screenshot`
- Cognos, Maestro'yu bir tool olarak cagirir (orchestrator uzerinden)

### 9.4 FridayX UI

Cognos, mevcut UI'a 3 yeni bilesen ekler:
1. Chat modunda: ses butonu + gorev ilerleme bari + karar seviyesi badge
2. Companion overlay: ayri window (NSPanel)
3. Menu bar ikonu: background mod durumu

---

## 10. Basari Metrikleri

| Metrik | Hedef | Olcum Yontemi |
|--------|-------|---------------|
| Task success rate | > %90 | Onaylanan gorevlerde basari |
| Halucinasyon orani | < %5 | Kaynak dogrulama bazli |
| P95 yanit suresi (L0-L1) | < 50ms | Local inference zamani |
| P95 yanit suresi (L2) | < 500ms | Ollama inference zamani |
| P95 yanit suresi (L3) | < 5s | Cloud LLM zamani |
| Local cozum orani | > %70 | L0+L1'de cozulen isteklerin orani |
| Cost per request (ortalama) | < $0.005 | API maliyeti / toplam istek |
| Kullanici geri bildirim skoru | > 4/5 | Explicit feedback |
| Checkpoint recovery | %100 | Gorev kaybi yok |
| Izin ihlali | 0 | Deny-default garanti |
| Ogrenme iyilestirmesi | > %10/ay | Aylik basari orani artisi |

---

## 11. Riskler ve Onlemler

| Risk | Etki | Onlem |
|------|------|-------|
| API maliyet patlamasi | Yuksek | Budget limiti + local-first + bandit optimizasyonu |
| Halucinasyon | Orta | Kaynak dogrulama + "bilmiyorum" politikasi |
| PII sizintisi | Yuksek | Local-first + maskeleme + audit trail |
| Local model kalitesi | Orta | Confidence threshold + LLM fallback |
| Companion App baglanti kopmasi | Dusuk | FCM offline queue + Supabase retry |
| Maestro/Pencil MCP kopmasi | Orta | Health check + graceful fallback |
| Uzun gorev timeout | Orta | Checkpoint + resume mekanizmasi |
| Yanlis form doldurma | Yuksek | Asla otomatik submit yok + kullanici kontrolu |
| LoRA overfitting | Orta | Validation set + early stopping |
| Adversarial input | Yuksek | Input sanitization + anomali tespiti |

---

## 12. Kapsam Disi

Bu modülde **olmayacak** seyler:
- Codex app-server akisini degistirmek
- Foundation model egitmek (sadece LoRA adapter)
- Odeme / finansal islem yapmak
- Baska kullanicilar icin calismak (tek kullanici sistemi)
- Workspace disi dosya silmek
- Kullanici adina form submit etmek
- Credential loglamak

---

## 13. Referanslar

- `docs/fridex-cognos-os-spec.tex` — Cognos-OS kanonik spesifikasyon (Active Inference, POMDP, VSA)
- `docs/ai-providers.md` — AI provider entegrasyon mimarisi
- `docs/maestro-mvp.md` — Maestro test runner MVP dokumantasyonu
- `docs/roadmap/ai/own-ai.md` — LLM olgunlasma ve olcekleme notlari
- `docs/zed-yol-haritasi.md` — Zed esinli editor yol haritasi

---

# KISIM II: PLATFORM ALTYAPISI VE ENTEGRASYON

---

## 14. Ses Sistemi: Karakter, Teknoloji ve Secim

### 14.1 TTS (Text-to-Speech) Secenekleri

| Secenek | Kalite | Maliyet | Latency | Privacy | Offline | Model Boyutu |
|---------|--------|---------|---------|---------|---------|-------------|
| **macOS System Voice** | Robotik, tanınabilir | Ucretsiz | <100ms | Tam local | Evet | 0 (sistem) |
| **Apple Personal Voice** | Kullanicinin kendi sesi | Ucretsiz | <200ms | Tam local | Evet | ~1GB |
| **Coqui XTTS v2** | Dogala yakin, voice clone | Ucretsiz | 300-500ms | Tam local | Evet | ~1.5GB |
| **Bark** (Suno AI) | Dogal, ekspresif | Ucretsiz | 500-1000ms | Tam local | Evet | ~5GB |
| **ElevenLabs** | En dogal | $5-22/ay | 200-400ms | Cloud | Hayir | 0 (API) |
| **OpenAI TTS** | Cok dogal | $15/1M char | 300-500ms | Cloud | Hayir | 0 (API) |

### 14.2 Onerilen Katmanli Yaklasim

```
Faz 0-3 (baslangic):
  → macOS AVSpeechSynthesizer
  → Sifir maliyet, sifir latency, offline
  → Siri'nin kullandigi motorun aynisi
  → Yeterli, mukemmel degil ama islevsel

Faz 4-5 (gecis):
  → Coqui XTTS v2 (local, open source)
  → 6 saniyelik ses ornegiyle voice cloning
  → Kendi sectigin bir ses karakteri yarat
  → Apple Silicon uzerinde ~300ms latency
  → Tamamen offline, privacy korunur

Opsiyonel (ozel durumlar):
  → ElevenLabs API (sunum, demo icin)
  → En dogal ses gerektiginde
  → Gunluk kullanim icin pahali
```

### 14.3 Ses Karakteri Karari

JARVIS/FRIDAY vizyonunda sesin bir **kimlik** tasimasi gerekir:

| Karar | Secenekler | Not |
|-------|-----------|-----|
| Cinsiyet | Erkek (JARVIS) / Kadin (FRIDAY) / Notr | Kullanici tercihi |
| Dil | Turkce / Ingilizce / Cift dilli | Whisper ikisini de destekler |
| Ton | Profesyonel-sakin / Samimi-sicak / Notr | XTTS v2 referans sesiyle belirlenir |
| Adaptasyon | Baglama gore ton degisimi | Acil → ciddi, casual → rahat |

**Cift Dilli Strateji:**
- Kullanici Turkce yazarsa → Turkce yanit + Turkce TTS
- Kullanici Ingilizce yazarsa → Ingilizce yanit + Ingilizce TTS
- Kod aciklamalari → Kullanici tercihine gore
- Intent classifier dil tespiti yapar (ek cikti: `lang=tr|en`)

### 14.4 STT (Speech-to-Text) Detaylari

| Model | Boyut | Dogruluk | Latency | RAM |
|-------|-------|----------|---------|-----|
| Whisper tiny | 39MB | Dusuk | <1s | ~200MB |
| Whisper small | 244MB | Orta | 1-2s | ~500MB |
| Whisper medium | 769MB | Yuksek | 2-4s | ~1.5GB |
| Whisper large-v3 | 1.5GB | En yuksek | 4-8s | ~3GB |

**Oneri:** `whisper small` ile basla (yeterli dogruluk, dusuk RAM). Gerekirse `medium`'a gec.

**mlx-whisper** (Apple MLX uzerinde): Ayni modeller ~2x hizli, Apple Silicon optimize.

### 14.5 Always-On Mikrofon Politikasi

| Mod | Davranis | Privacy Riski |
|-----|----------|---------------|
| **Push-to-talk** (varsayilan) | Kullanici butona basinca dinler | Sifir |
| **Wake word** ("Hey Friday") | Sadece wake word icin dinler, sonra kayit baslar | Dusuk |
| **Always-on** | Surekli dinler, VAD ile konusma tespit eder | Yuksek |
| **App-only** | Sadece FridayX/Companion app acikken dinler | Dusuk |

**Oneri:** Push-to-talk ile basla. Wake word Faz 10'da opsiyonel olarak ekle.

---

## 15. Cognos Companion App (Mobil Bildirim ve Iletisim)

WhatsApp/Telegram'a bagimlilik yerine, kendi companion app'imiz.

### 15.1 Neden Kendi App?

| WhatsApp Business API | Cognos Companion App |
|----------------------|---------------------|
| Kurumsal onay gerektirir | Sadece EAS staging build, store onayı yok |
| Mesaj formati sinirli | Zengin UI: butonlar, formlar, kod bloklari |
| API maliyeti var ($0.005/mesaj) | Ucretsiz (kendi sunucun) |
| Ucuncu parti bagimlilik | Tam kontrol |
| Rate limit var | Limit yok |
| Sadece metin + resim | Ses, video, interaktif onay paneli |

### 15.2 Teknik Mimari

```
┌─────────────────────────┐         ┌─────────────────────────┐
│   FridayX (Desktop)      │         │  Cognos Companion (Mobil)│
│   Tauri + React          │         │  Expo + React Native     │
│                          │         │                          │
│  Orchestrator ──────────────WS────────── Push Handler         │
│  Task Queue   ──────────────────────── Notification Center    │
│  World Model             │         │  Mini Chat UI            │
│                          │         │  Onay/Red Butonlari      │
│                          │         │  Ses Girisi (PTT)        │
│                          │         │  Gorev Ilerleme          │
└─────────────────────────┘         └─────────────────────────┘
         │                                    │
         └──────────── Supabase / ──────────┘
                    Firebase Cloud Messaging
                    (veya kendi WebSocket sunucusu)
```

### 15.3 Companion App Ozellikleri

**Bildirim Tipleri:**
```
┌─────────────────────────────────────┐
│  🔵 Cognos                    14:32  │
│                                      │
│  "auth.ts'deki login fix'i icin      │
│   2 yaklasim buldum:                 │
│                                      │
│   A) Token refresh mekanizmasi       │
│   B) Session middleware yenileme     │
│                                      │
│   Hangisini uygulayayim?"            │
│                                      │
│  ┌──────────┐  ┌──────────┐         │
│  │  A Secimi │  │  B Secimi │         │
│  └──────────┘  └──────────┘         │
│                                      │
│  ┌──────────────────────────┐        │
│  │  💬 Mesaj yaz...          │        │
│  └──────────────────────────┘        │
└─────────────────────────────────────┘
```

**Mini Chat:**
- Tam chat degil, sadece Cognos'un sordugu sorulara yanit
- Quick reply butonlari (Evet/Hayir, A/B/C secimi)
- Sesli yanit (push-to-talk, Whisper ile STT)
- Gorev ilerleme bari

**Gorev Dashboard:**
- Aktif gorevlerin durumu (pending/running/blocked/done)
- Her gorev icin kisa ozet
- "Devam et" / "Iptal" butonlari

### 15.4 Build ve Dagitim

```
Teknoloji: Expo (React Native)
  → Mevcut React/TypeScript bilgisiyle hizli gelistirme
  → EAS Build ile staging build
  → TestFlight (iOS) veya dogrudan APK (Android)
  → Store'a cikmaya gerek yok

Build komutu:
  eas build --profile staging --platform ios
  eas build --profile staging --platform android

Guncelleme:
  eas update --branch staging
  → OTA (Over-the-Air) guncelleme, yeni build gerekmez

Maliyet: $0 (Expo ucretsiz tier yeterli, tek kullanici)
```

### 15.5 Desktop ↔ Mobil Iletisim Protokolu

**Secenek A: Firebase Cloud Messaging (FCM) — Onerilen**
```
Desktop → FCM HTTP API → Push notification → Companion App
Companion App → WebSocket → Desktop
```
- Ucretsiz (Google Firebase free tier)
- Guvenilebilir push notification
- Uygulama kapali olsa bile bildirim gelir

**Secenek B: Kendi WebSocket Sunucusu**
```
Desktop → WS Server (VPS veya Cloudflare Workers) → Companion App
```
- Tam kontrol
- Ek sunucu maliyeti (~$5/ay VPS)
- Uygulama acik olmali (background icin push yine lazim)

**Secenek C: Supabase Realtime**
```
Desktop → Supabase Realtime Channel → Companion App
```
- Ucretsiz tier yeterli
- Realtime subscriptions + database
- Auth dahil

**Oneri:** Firebase Cloud Messaging (push) + Supabase Realtime (iki yonlu chat). Ikisi de ucretsiz tier'da yeterli.

### 15.6 Guvenlik

- End-to-end encryption: desktop ↔ mobil arasinda sifreleme
- Device pairing: QR kod ile eslesme (ilk kurulumda)
- Session token: her oturumda yenilenen JWT
- Biometric: companion app'e giris Face ID / Touch ID ile
- Uzaktan kilitleme: desktop'tan companion app'i devre disi birakma

---

## 16. Sistem Gereksinimleri ve Maliyet Analizi

### 16.1 Donanim Gereksinimleri

**Minimum (Faz 0-3, LLM Wrapper + Basic NLP):**

| Bilesen | Gereksinim | Not |
|---------|-----------|-----|
| Islemci | Apple Silicon M1+ | Neural Engine zorunlu |
| RAM | 16GB | Ollama 7B + Whisper small + sistem |
| Depolama | 256GB+ SSD | Modeller ~10GB yer kaplar |
| Mikrofon | Dahili yeterli | Push-to-talk icin |
| Ag | Baslangicta internet gerekli | Cloud LLM'ler icin |

**Model Bellek Kullanimi (Faz 0-3):**
```
Whisper small  :  ~500MB
Intent clf     :  ~250MB
Ollama 7B Q4   : ~4.0GB
Vector store   :  ~100MB
SQLite + KG    :   ~50MB
─────────────────────────
Toplam         : ~4.9GB  → 16GB RAM'de rahat calısır
```

**Ideal (Faz 4-10, Full Cognitive System):**

| Bilesen | Gereksinim | Not |
|---------|-----------|-----|
| Islemci | Apple Silicon M3 Pro+ / M4 | Daha guclu Neural Engine |
| RAM | 32GB+ (ideal: 64GB) | Buyuk local modeller icin |
| Depolama | 512GB+ SSD | Modeller + egitim verisi ~30GB |
| Mikrofon | Podcast kalitesi harici | Whisper dogrulugu artar |
| Ag | Opsiyonel | Cogunluk local'de cozulur |

**Model Bellek Kullanimi (Faz 4-10):**
```
Whisper medium : ~1.5GB
Intent + NER   :  ~500MB
Sentiment      :  ~250MB
XTTS v2 (TTS)  : ~1.5GB
Ollama 13B Q4  : ~8.0GB
GNN + KG embed :  ~500MB
Vector store   :  ~500MB
LoRA adapters  :  ~200MB
─────────────────────────
Toplam         : ~13.0GB → 32GB RAM'de rahat, 16GB'da dar
```

### 16.2 Yazilim Gereksinimleri

```
KATMAN 0 — Zaten var (FridayX mevcut):
├── Tauri 2 + Rust backend
├── React 19 + Vite frontend
├── Node.js runtime
├── SQLite (Tauri icinde)
└── Xcode Command Line Tools

KATMAN 1 — Faz 0 icin eklenecek:
├── Ollama                         → brew install ollama
│   ├── codellama:7b-q4            → ollama pull codellama:7b
│   └── llama3.2:3b                → ollama pull llama3.2:3b
├── Claude CLI                     → zaten entegre
└── Gemini CLI                     → zaten entegre

KATMAN 2 — Faz 4 icin eklenecek (Local NLP):
├── whisper.cpp veya whisper-rs     → Cargo dependency
├── candle (Rust ML, HuggingFace)   → Cargo dependency
│   └── DistilBERT model            → HF'den indir (~250MB)
├── silero-vad                      → ONNX model (~5MB)
├── ort (ONNX Runtime for Rust)     → Cargo dependency
└── tokenizers (HF Rust)            → Cargo dependency

KATMAN 3 — Faz 5 icin eklenecek (Ses):
├── Coqui XTTS v2                   → Python sidecar veya mlx-audio
├── macOS AVSpeechSynthesizer       → Native Swift/ObjC bridge
└── Core Audio (mic capture)        → Tauri plugin

KATMAN 4 — Faz 8 icin eklenecek (Knowledge Graph):
├── oxigraph                        → Cargo dependency (embedded)
└── usearch veya hnsw_rs            → Cargo dependency (vector index)

KATMAN 5 — Faz 9 icin eklenecek (Ogrenme):
├── MLX (Apple ML framework)        → pip install mlx
│   └── mlx-lm (LoRA fine-tuning)   → pip install mlx-lm
└── Core ML Tools                   → Model export pipeline

KATMAN 6 — Companion App:
├── Expo / React Native             → npx create-expo-app
├── EAS CLI                         → npm install -g eas-cli
├── Firebase (push notification)    → expo-notifications
└── Supabase (realtime + auth)      → @supabase/supabase-js
```

### 16.3 Maliyet Analizi

**Faz 0-3 (ilk 6 ay): TOPLAM $0**

| Kalem | Maliyet | Not |
|-------|---------|-----|
| Donanim | $0 | Mevcut Mac yeterli |
| Yazilim | $0 | Hepsi acik kaynak |
| Cloud LLM | $0 | Claude CLI + Gemini CLI ucretsiz |
| Local LLM | $0 | Ollama ucretsiz |
| TTS | $0 | macOS system voice |
| Veritabani | $0 | SQLite (embedded) |
| **Toplam** | **$0** | |

**Faz 4-7 (6-18 ay): TOPLAM ~$50-200/yil**

| Kalem | Maliyet | Not |
|-------|---------|-----|
| ElevenLabs (opsiyonel) | $60/yil | Premium TTS, sadece gerekirse |
| Firebase (push) | $0 | Ucretsiz tier yeterli |
| Supabase (realtime) | $0 | Ucretsiz tier yeterli |
| Expo EAS (build) | $0 | Ucretsiz tier (30 build/ay) |
| Mikrofon (opsiyonel) | $50-100 | Tek seferlik |
| VPS (opsiyonel) | $60/yil | Sadece kendi WS sunucusu istersen |
| **Toplam** | **~$50-200** | Cogu opsiyonel |

**Faz 8-10 (18-24 ay): TOPLAM ~$0-360/yil**

| Kalem | Maliyet | Not |
|-------|---------|-----|
| Cloud API (fallback) | <$30/ay | Cogunluk local, API nadir |
| LoRA egitim | $0 | Local, elektrik maliyeti ihmal edilir |
| Depolama | $0 | Local SSD |
| **Toplam** | **~$0-360/yil** | Bandit optimizasyonu ile duser |

**Kritik nokta:** Sistemin cekirdegi **sifir bulut maliyetiyle** calisabilir. Claude CLI ve Gemini CLI ucretsiz. Ollama tamamen ucretsiz. Companion app EAS staging build ile store'a cikmadan kullanilir.

### 16.4 Olceklenebilirlik Notu

Bu sistem **tek kullanici** icin tasarlanmistir. Olcekleme hedefi yok. Bu, mimariyi basitlestiren kritik bir karardir:
- Multi-tenant izolasyonu gerekmez
- Rate limiting sadece API maliyeti icin
- Veritabani olceklemesi gerekmez (SQLite yeterli)
- Companion app tek cihazda calisir

---

## 17. Acik Kararlar ve Tartisma Konulari

Implementasyona baslamadan once netlestirilmesi gereken noktalar:

| # | Karar | Secenekler | Varsayilan |
|---|-------|-----------|-----------|
| 1 | Ses karakteri cinsiyeti | Erkek (JARVIS) / Kadin (FRIDAY) / Notr | Kullanici belirler |
| 2 | Birincil dil | Turkce / Ingilizce / Cift dilli | Cift dilli (otomatik tespit) |
| 3 | Mikrofon politikasi | Push-to-talk / Wake word / Always-on | Push-to-talk |
| 4 | Companion app platform | iOS only / Android only / Her ikisi | iOS (EAS staging) |
| 5 | Desktop ↔ Mobil iletisim | FCM + Supabase / Kendi WS / Sadece FCM | FCM + Supabase |
| 6 | Baslangic local LLM | codellama:7b / llama3.2:3b / mistral:7b | llama3.2:3b (hizli) |
| 7 | Knowledge Graph DB | oxigraph / SQLite JSON / Custom Rust | oxigraph |
| 8 | XTTS referans sesi | Secilecek ses ornegi | Faz 4'te belirlenir |
| 9 | Cognos ismi | Cognos / Friday / Ozel isim | Kullanici belirler |

---

# KISIM III: BILISSEL EVRIM — LLM WRAPPER'DAN KOGNITIF SISTEME

---

## 18. Algi Pipeline'i (Perception)

JARVIS komut beklemez, surekli algilar. Cognos'un "duyulari":

### 18.1 Ses Islemcisi

#### 18.1.1 Ses Aktivite Tespiti (VAD — Voice Activity Detection)

Ham ses sinyali $x[n]$ uzerinde enerji tabanli VAD:

$$E_{\text{frame}}(k) = \sum_{n=kH}^{kH+N-1} x[n]^2 \cdot w[n - kH]$$

Burada:
- $N$: pencere boyutu (tipik: 25ms = 400 sample @ 16kHz)
- $H$: hop boyutu (tipik: 10ms = 160 sample)
- $w[n]$: Hamming pencere fonksiyonu: $w[n] = 0.54 - 0.46 \cos\left(\frac{2\pi n}{N-1}\right)$

**Karar:**
$$\text{is\_speech}(k) = \begin{cases} 1 & \text{if } E_{\text{frame}}(k) > \theta_{\text{adaptive}} \\ 0 & \text{otherwise} \end{cases}$$

Adaptif esik:
$$\theta_{\text{adaptive}} = \alpha \cdot \theta_{\text{adaptive}} + (1-\alpha) \cdot \text{percentile}(E_{\text{recent}}, 90)$$

> **Uretim notu:** silero-vad (ONNX, <5MB) bu hesaplamayi optimize edilmis bir sinir agi ile yapar. Yukaridaki matematik, fallback veya debug icin kullanilir.

#### 18.1.2 MFCC — Mel-Frequency Cepstral Coefficients

Ses sinyalinden ozellik cikarimi (intent classifier, duygu analizi icin):

**Adim 1: Guc Spektrumu**
$$P(k, f) = |X(k, f)|^2 \quad \text{where } X = \text{STFT}(x)$$

**Adim 2: Mel Filtre Bankasi**
Mel olcegi insan kulak algisinı modeller:
$$m = 2595 \cdot \log_{10}\left(1 + \frac{f}{700}\right)$$

Ters donusum:
$$f = 700 \cdot \left(10^{m/2595} - 1\right)$$

$M$ adet ucgen filtre bankasi $H_m(f)$ ile:
$$S(k, m) = \sum_{f} P(k, f) \cdot H_m(f) \quad m = 1, 2, \ldots, M$$

**Adim 3: Log + DCT**
$$\text{MFCC}(k, c) = \sum_{m=1}^{M} \log(S(k, m)) \cdot \cos\left(\frac{\pi c (m - 0.5)}{M}\right)$$

Tipik olarak ilk 13 MFCC katsayisi alinir ($c = 1, \ldots, 13$).

#### 18.1.3 CTC Loss — Whisper/STT Egitimi

Connectionist Temporal Classification, degisken uzunluktaki ses → metin hizalamasi icin:

$$P(\mathbf{y} | \mathbf{x}) = \sum_{\boldsymbol{\pi} \in \mathcal{B}^{-1}(\mathbf{y})} \prod_{t=1}^{T} P(\pi_t | \mathbf{x})$$

Burada:
- $\mathbf{y}$: hedef metin dizisi
- $\boldsymbol{\pi}$: olasi hizalama yolu (blank tokenler dahil)
- $\mathcal{B}^{-1}(\mathbf{y})$: $\mathbf{y}$'ye eslesen tum gecerli yollar
- $T$: zaman adimlari

**Kayip:**
$$\mathcal{L}_{\text{CTC}} = -\ln P(\mathbf{y} | \mathbf{x})$$

Forward-backward algoritmasi ile verimli hesaplanir: $O(T \cdot |\mathbf{y}|)$.

> **Uretim notu:** Whisper.cpp zaten egitilmis bir model kullanir. CTC math, ileride custom STT adapter egitimi icin referanstir.

### 18.2 Ekran Farkindaligi

macOS Accessibility API uzerinden:

| Bilgi | API | Guncelleme Sikligi |
|-------|-----|-------------------|
| Aktif uygulama | `NSWorkspace.shared.frontmostApplication` | Degisimde |
| Pencere basligi | `AXUIElement` API | 1s polling |
| Kullanici idle suresi | `CGEventSourceSecondsSinceLastEventType` | 5s polling |
| Ekran icerigi | `CGWindowListCreateImage` (screenshot) | Gerektiginde |

**Idle durum tahmini:**
$$P(\text{idle}) = \sigma\left(\frac{t_{\text{idle}} - \mu_{\text{idle}}}{\tau}\right) = \frac{1}{1 + e^{-(t_{\text{idle}} - \mu_{\text{idle}})/\tau}}$$

- $\mu_{\text{idle}}$: kullanicinin ortalama idle suresi (ogrenilebilir)
- $\tau$: sicaklik parametresi

### 18.3 Sistem Telemetrisi

| Sinyal | Kaynak | Bilgi Degeri |
|--------|--------|-------------|
| CPU/RAM | `sysinfo` crate | Agir islem calisiyor mu? |
| Disk I/O | filesystem watcher | Build/compile calisiyor mu? |
| Network | netstat | Download/upload aktif mi? |
| Pil | `battery` crate | Guc tasarrufu modu gerekli mi? |
| Git | filesystem watcher | Branch degisti mi? Commit yapildi mi? |

### 18.4 Algi Fuzyonu (Sensor Fusion)

Birden fazla algi kanalini birlestirmek icin **Bayesian Sensor Fusion**:

$$P(s | o_1, o_2, \ldots, o_K) \propto P(s) \prod_{k=1}^{K} P(o_k | s)$$

Ornek:
- $o_1$: Kullanici hizli yaziyor (keyboard telemetri)
- $o_2$: Terminal'de hata mesaji var (ekran farkindaligi)
- $o_3$: "neden" kelimesini iceren sorgu (metin analizi)

$$P(\text{debugging} | o_1, o_2, o_3) \propto P(\text{debugging}) \cdot P(\text{hizli\_yazim} | \text{debugging}) \cdot P(\text{hata\_mesaji} | \text{debugging}) \cdot P(\text{"neden"} | \text{debugging})$$

Her kanal bagimsiz ama birlesimleri guclu bir durum tahmini verir.

---

## 19. Local NLP Pipeline — Derin Ogrenme Matematigi

### 19.1 Transformer Mimarisi (Intent Classifier ve NER icin)

#### 19.1.1 Self-Attention Mekanizmasi

Girdi dizisi $\mathbf{X} \in \mathbb{R}^{n \times d}$ icin:

$$\mathbf{Q} = \mathbf{X}\mathbf{W}_Q, \quad \mathbf{K} = \mathbf{X}\mathbf{W}_K, \quad \mathbf{V} = \mathbf{X}\mathbf{W}_V$$

$$\text{Attention}(\mathbf{Q}, \mathbf{K}, \mathbf{V}) = \text{softmax}\left(\frac{\mathbf{Q}\mathbf{K}^\top}{\sqrt{d_k}}\right)\mathbf{V}$$

Burada:
- $\mathbf{W}_Q, \mathbf{W}_K \in \mathbb{R}^{d \times d_k}$, $\mathbf{W}_V \in \mathbb{R}^{d \times d_v}$: ogrenilen projeksiyon matrisleri
- $d_k$: anahtar boyutu
- $\sqrt{d_k}$: olcekleme faktoru (gradient stabilitesi icin)

**Multi-Head Attention:**
$$\text{MultiHead}(\mathbf{Q}, \mathbf{K}, \mathbf{V}) = \text{Concat}(\text{head}_1, \ldots, \text{head}_h)\mathbf{W}_O$$

$$\text{head}_i = \text{Attention}(\mathbf{X}\mathbf{W}_Q^i, \mathbf{X}\mathbf{W}_K^i, \mathbf{X}\mathbf{W}_V^i)$$

**Karmasiklik:** $O(n^2 \cdot d)$ — girdi uzunlugunun karesiyle orantili.

> **Uretim notu:** DistilBERT (66M parametre) yeterlidir. $n \leq 128$ token ile <10ms inference (Apple Neural Engine).

#### 19.1.2 Position Encoding

Dizi sirasini kodlamak icin sinuzoidal pozisyon kodlamasi:

$$\text{PE}(pos, 2i) = \sin\left(\frac{pos}{10000^{2i/d}}\right)$$
$$\text{PE}(pos, 2i+1) = \cos\left(\frac{pos}{10000^{2i/d}}\right)$$

### 19.2 Intent Classifier

#### 19.2.1 Model Mimarisi

```
Input Tokens → DistilBERT Encoder → [CLS] Token → Dense(768, 256) → ReLU → Dropout(0.1) → Dense(256, K) → Softmax
```

$K$: intent sinif sayisi.

**Softmax:**
$$P(y = k | \mathbf{x}) = \frac{e^{z_k}}{\sum_{j=1}^{K} e^{z_j}}$$

**Egitim kayip fonksiyonu (Cross-Entropy):**
$$\mathcal{L}_{\text{CE}} = -\sum_{k=1}^{K} y_k \cdot \ln P(y = k | \mathbf{x})$$

#### 19.2.2 Intent Sinif Listesi

| Intent ID | Sinif | Ornek |
|-----------|-------|-------|
| 0 | `code_write` | "bu fonksiyonu yaz" |
| 1 | `code_fix` | "bu bug'i duzelt" |
| 2 | `code_explain` | "bu kod ne yapiyor?" |
| 3 | `test_run` | "testleri calistir" |
| 4 | `test_write` | "bu icin test yaz" |
| 5 | `search` | "su konuyu arastir" |
| 6 | `git_op` | "commit yap" |
| 7 | `design` | "bu ekrani tasarla" |
| 8 | `question` | "TypeScript'te X nasil yapilir?" |
| 9 | `conversation` | "merhaba, nasilsin" |
| 10 | `command` | "terminali ac" |
| 11 | `form_fill` | "vize formunu doldur" |
| 12 | `notify` | "bana haber ver" |

#### 19.2.3 Confidence Threshold ve Escalation

$$\text{decision} = \begin{cases} \text{L0: Pattern Match} & \text{if rule-based match} \\ \text{L1: Local Action} & \text{if } \max_k P(y=k|\mathbf{x}) > \theta_{\text{high}} \\ \text{L2: Ollama} & \text{if } \max_k P(y=k|\mathbf{x}) > \theta_{\text{low}} \\ \text{L3: Cloud LLM} & \text{otherwise} \end{cases}$$

Tipik degerler: $\theta_{\text{high}} = 0.85$, $\theta_{\text{low}} = 0.5$.

### 19.3 Named Entity Recognition (NER)

#### 19.3.1 BIO Tagging Semasi

```
Kullanici: "auth.ts dosyasindaki login fonksiyonunu duzelt"

Tokenler:  [auth.ts]      [dosyasindaki] [login]       [fonksiyonunu] [duzelt]
BIO Tags:  [B-FILE]       [O]            [B-FUNCTION]  [O]            [O]
```

#### 19.3.2 Conditional Random Field (CRF)

NER icin cikis katmaninda CRF kullanilir. Dizi etiketleme olasiligi:

$$P(\mathbf{y} | \mathbf{x}) = \frac{1}{Z(\mathbf{x})} \exp\left(\sum_{t=1}^{T} \left(E(y_t, \mathbf{x}, t) + T(y_{t-1}, y_t)\right)\right)$$

Burada:
- $E(y_t, \mathbf{x}, t)$: emisyon skoru (transformer'dan gelen logit)
- $T(y_{t-1}, y_t)$: gecis skoru (ogrenilen etiket-etiket matrisi)
- $Z(\mathbf{x})$: normalizasyon sabiti (partition function)

**Partition function (forward algorithm ile):**
$$Z(\mathbf{x}) = \sum_{\mathbf{y}'} \exp\left(\sum_{t} E(y'_t, \mathbf{x}, t) + T(y'_{t-1}, y'_t)\right)$$

**Viterbi Decoding (en olasi etiket dizisi):**
$$\mathbf{y}^* = \arg\max_{\mathbf{y}} P(\mathbf{y} | \mathbf{x})$$

Dinamik programlama ile $O(T \cdot K^2)$ karmasiklikta cozulur ($K$: etiket sayisi).

### 19.4 Duygu/Ton Analizi (Sentiment)

#### 19.4.1 Metin Bazli

Ayni DistilBERT encoder + ayri classification head:

$$P(\text{sentiment} = s | \mathbf{x}) = \text{softmax}(\mathbf{W}_s \cdot \mathbf{h}_{\text{[CLS]}} + \mathbf{b}_s)$$

Siniflar: `neutral`, `frustrated`, `urgent`, `casual`, `focused`, `confused`

#### 19.4.2 Ses Bazli (Paralinguistic Features)

Ses tonundan duygu cikarimi icin ek ozellikler:

| Ozellik | Formul | Anlam |
|---------|--------|-------|
| Pitch (F0) | Autocorrelation yontemi | Stres/heyecan gostergesi |
| Konusma hizi | $\text{syllables} / \text{duration}$ | Aciliyet gostergesi |
| Enerji varyans | $\text{Var}(E_{\text{frame}})$ | Duygusal yogunluk |
| Spectral centroid | $\frac{\sum f \cdot P(f)}{\sum P(f)}$ | Ses "parlaklik" tonu |

**Fusion (metin + ses):**
$$P(\text{emotion} | \text{text}, \text{audio}) = \sigma(\mathbf{w}_t^\top \mathbf{f}_{\text{text}} + \mathbf{w}_a^\top \mathbf{f}_{\text{audio}} + b)$$

### 19.5 Dialogue State Tracker (DST)

Konusma durumunu yapisal olarak takip eder (sadece "son N mesaj" degil):

**Durum temsili:**
$$\mathbf{s}_t = (\text{goal}_t, \text{subgoals}_t, \text{entities}_t, \text{pending\_questions}_t, \text{user\_state}_t)$$

**Guncelleme (GRU tabanlı):**
$$\mathbf{z}_t = \sigma(\mathbf{W}_z [\mathbf{h}_{t-1}, \mathbf{x}_t])$$
$$\mathbf{r}_t = \sigma(\mathbf{W}_r [\mathbf{h}_{t-1}, \mathbf{x}_t])$$
$$\tilde{\mathbf{h}}_t = \tanh(\mathbf{W} [\mathbf{r}_t \odot \mathbf{h}_{t-1}, \mathbf{x}_t])$$
$$\mathbf{h}_t = (1 - \mathbf{z}_t) \odot \mathbf{h}_{t-1} + \mathbf{z}_t \odot \tilde{\mathbf{h}}_t$$

- $\mathbf{z}_t$: update gate (ne kadar yeni bilgi alalim?)
- $\mathbf{r}_t$: reset gate (eski bilgiyi ne kadar unutalim?)
- $\odot$: element-wise carpim

---

## 20. Surekli Ogrenme Pipeline'i (Continuous Learning)

### 20.1 Tercih Ogrenimi (Preference Learning)

#### 20.1.1 Bradley-Terry Modeli

Kullanici iki yanit arasinda tercih yaptiginda:

$$P(y_w \succ y_l | \mathbf{x}) = \sigma(r_\theta(y_w, \mathbf{x}) - r_\theta(y_l, \mathbf{x}))$$

Burada:
- $y_w$: tercih edilen yanit, $y_l$: reddedilen yanit
- $r_\theta$: reward model (ogrenilecek)
- $\sigma$: sigmoid fonksiyonu

**Kayip:**
$$\mathcal{L}_{\text{pref}} = -\mathbb{E}_{(y_w, y_l)} [\ln \sigma(r_\theta(y_w) - r_\theta(y_l))]$$

#### 20.1.2 Implicit Feedback

Kullanici acikca tercih belirtmese bile:
- AI'nin kodunu duzenlediyse → duzenleme farki = negatif sinyal
- Oneriyi kabul ettiyse → pozitif sinyal
- Oneriyi gormezden geldiyse → hafif negatif sinyal

$$r_{\text{implicit}} = \begin{cases} +1.0 & \text{accepted as-is} \\ +0.5 & \text{accepted with minor edits} \\ -0.3 & \text{ignored} \\ -0.7 & \text{rejected / major rewrite} \\ -1.0 & \text{explicit negative feedback} \end{cases}$$

### 20.2 LoRA — Low-Rank Adaptation

Foundation model agirliklarini dondurup, kucuk adapter matrisleri eklenir:

$$\mathbf{W}' = \mathbf{W}_0 + \Delta\mathbf{W} = \mathbf{W}_0 + \mathbf{B}\mathbf{A}$$

Burada:
- $\mathbf{W}_0 \in \mathbb{R}^{d \times k}$: orijinal (dondurulmus) agirlik matrisi
- $\mathbf{B} \in \mathbb{R}^{d \times r}$, $\mathbf{A} \in \mathbb{R}^{r \times k}$: dusuk rank'li adapter matrisleri
- $r \ll \min(d, k)$: rank (tipik: $r = 8$ veya $r = 16$)

**Parametre tasarrufu:**
$$\text{Orijinal: } d \times k \text{ parametre}$$
$$\text{LoRA: } d \times r + r \times k = r(d + k) \text{ parametre}$$

$d = k = 4096$, $r = 16$ icin: $4096^2 = 16.7M$ yerine $2 \times 4096 \times 16 = 131K$ parametre. **%99.2 tasarruf.**

**QLoRA (Quantized LoRA):**

Orijinal agirliklari 4-bit'e quantize edip uzerine LoRA eklenir:

$$\mathbf{W}_0^{\text{4bit}} = \text{NF4}(\mathbf{W}_0)$$
$$\mathbf{W}' = \text{dequant}(\mathbf{W}_0^{\text{4bit}}) + \mathbf{B}\mathbf{A}$$

NF4 (NormalFloat 4-bit): normal dagilim varsayimiyla optimal 4-bit quantization.

> **Uretim notu:** QLoRA ile 7B model ~4GB RAM ile fine-tune edilebilir. Apple M-series uzerinde MLX framework ile local egitim mumkun.

### 20.3 Knowledge Distillation (Bilgi Damitma)

Buyuk cloud model'in bilgisini kucuk local model'e aktarma:

**Soft Target Distillation:**

$$\mathcal{L}_{\text{distill}} = (1-\alpha) \cdot \mathcal{L}_{\text{CE}}(\mathbf{y}, \hat{\mathbf{y}}_S) + \alpha \cdot T^2 \cdot \mathrm{D}_{KL}(\hat{\mathbf{p}}_T^{(T)} \| \hat{\mathbf{p}}_S^{(T)})$$

Burada:
- $\hat{\mathbf{y}}_S$: ogrenci (student) model ciktisi
- $\hat{\mathbf{p}}_T^{(T)}$: ogretmen (teacher) model'in soft probabilities ($T$ sicaklikta)
- $T$: sicaklik parametresi (tipik: $T = 4$)
- $\alpha$: distillation agirligi (tipik: $\alpha = 0.7$)

**Soft probabilities:**
$$p_i^{(T)} = \frac{e^{z_i / T}}{\sum_j e^{z_j / T}}$$

Yuksek $T$ degeri, ogretmenin "belirsizlik bilgisini" de aktarir.

**Pratik akis:**
```
1. Cloud LLM (Claude Opus) bir gorevi cozuyor → cikti + reasoning
2. Bu cikti (input, output) cifti olarak kaydedilir
3. N adet birikmis ornek ile local model LoRA ile fine-tune edilir
4. Sonraki benzer gorev local model'e yonlendirilir
5. Basari olcumleri ile distillation kalitesi dogrulanir
```

### 20.4 Online Ogrenme — Exponential Moving Average (EMA)

Model agirliklari surekli guncellenir (catastrophic forgetting'i azaltmak icin):

$$\boldsymbol{\theta}_{\text{EMA}} = \beta \cdot \boldsymbol{\theta}_{\text{EMA}} + (1-\beta) \cdot \boldsymbol{\theta}_{\text{new}}$$

Tipik: $\beta = 0.999$ (yavas, stabil guncelleme).

### 20.5 Beceri Kazanimi — Progressive Skill Transfer

```
Gorev ilk kez:    Cloud LLM (tam reasoning)    → Maliyet: $$$
Gorev 2. kez:     Cached strateji + Local LM   → Maliyet: $$
Gorev 3-5. kez:   Local model (LoRA adapted)   → Maliyet: $
Gorev 5+ kez:     Local small model (distilled) → Maliyet: ¢
```

Her gorev turu icin bir "skill level" takip edilir:

$$\text{skill\_level}(task) = \min\left(1.0, \frac{\text{successful\_local\_completions}(task)}{\text{total\_attempts}(task)} \cdot \ln(1 + \text{total\_attempts})\right)$$

$\text{skill\_level} > 0.8$ oldugunda gorev tamamen local model'e devredilir.

---

## 21. Dunya Modeli (World Model)

### 21.1 Uc Katmanli Model

#### Katman 1: Knowledge Graph (statik bilgi)

Bolum 2.5'te tanimlanan $G = (V, E, \mathcal{A}, \mathcal{R})$ yapisi.

#### Katman 2: Temporal Model (zamansal oruntu)

Kullanici davranislarinin zamansal kaliplarini ogrenme:

**Periodic Pattern Detection (Fourier analizi):**

$$X(f) = \sum_{n=0}^{N-1} x[n] \cdot e^{-j2\pi fn/N}$$

Guclu frekans bilesenleri → periyodik davranislar:
- $f = 1/\text{gun}$: gunluk kaliplar (sabah kodlama, ogle arastirma, aksam planlama)
- $f = 1/\text{hafta}$: haftalik kaliplar (Pazartesi sprint, Cuma refactor)

**Hidden Markov Model (HMM) ile Aktivite Dizisi:**

Kullanicinin gizli durumlarini ($S$: coding, debugging, researching, idle, meeting) gozlemlerden cikar:

$$P(S_{1:T}, O_{1:T}) = P(S_1) \prod_{t=2}^{T} P(S_t | S_{t-1}) \prod_{t=1}^{T} P(O_t | S_t)$$

- $P(S_t | S_{t-1})$: gecis olasiliklari (Transition matrix $\mathbf{A}$)
- $P(O_t | S_t)$: emisyon olasiliklari (hangi durumda ne gozlemlenir)

**Ornek gecis matrisi:**

| | Coding | Debugging | Researching | Idle |
|---|---|---|---|---|
| **Coding** | 0.7 | 0.15 | 0.1 | 0.05 |
| **Debugging** | 0.2 | 0.5 | 0.2 | 0.1 |
| **Researching** | 0.3 | 0.1 | 0.5 | 0.1 |
| **Idle** | 0.4 | 0.1 | 0.2 | 0.3 |

Baum-Welch algoritmasi ile kullanici verisinden ogrenilir.

#### Katman 3: Predictive State (tahmin)

Bir sonraki eylemi tahmin etme:

$$P(a_{t+1} | \text{file}_t, \text{hour}_t, a_{t-2:t}, \text{project\_state}_t)$$

**Ornek:**
```
auth.ts acik + dun login bug issue acildi + saat 09:30
→ P(bug_fix) = 0.85
→ Proaktif: "Dunku login bug'i icin auth.ts'i actigin goruyorum.
   Ilgili test dosyasini da acayim mi?"
```

Tahmin modeli **lightweight LSTM** veya **attention-based sequence model** ile:

$$\mathbf{h}_t = \text{LSTM}([\mathbf{e}_{\text{file}}, \mathbf{e}_{\text{hour}}, \mathbf{e}_{a_{t-1}}], \mathbf{h}_{t-1})$$
$$P(a_{t+1}) = \text{softmax}(\mathbf{W}_a \mathbf{h}_t + \mathbf{b}_a)$$

### 21.2 Proaktif Oneri Sistemi

Dunya modeli yeterince olgunlastiginda (Faz 10), sistem proaktif onerilerde bulunur.

**Oneri skoru:**
$$\text{Proactive\_Score}(suggestion) = \text{confidence} \cdot \text{utility} \cdot (1 - \text{interruption\_cost})$$

- $\text{confidence}$: tahmin guvenilirligi (0-1)
- $\text{utility}$: onerinin kullaniciya faydasi (0-1)
- $\text{interruption\_cost}$: kullaniciyi rahatsiz etme maliyeti (0-1)
  - Deep focus modunda yuksek
  - Idle'da dusuk
  - Toplanti sirasinda maksimum

**Oneri esigi:**
$$\text{suggest\_if} \quad \text{Proactive\_Score} > \theta_{\text{proactive}}$$

$\theta_{\text{proactive}}$ kullanici geri bildirimine gore adapte edilir:
- Oneri kabul edildi → $\theta$ biraz dusur (daha fazla oner)
- Oneri reddedildi → $\theta$ biraz artir (daha az oner)
- Oneri "rahatsiz etti" → $\theta$ belirgin artir

---

## 22. Hiperboyutlu Hesaplama (VSA/HDC) — Verimli Hafiza Temsili

Cognos-OS spec'ten. $D \approx 10{,}000$ boyutlu ikili vektorlerle:

### 22.1 Temel Operasyonlar

| Operasyon | Formul | Anlam |
|-----------|--------|-------|
| **Binding (XOR)** | $\mathbf{u} \otimes \mathbf{v} \Rightarrow u_i \oplus v_i$ | Iki kavrami birlestir |
| **Bundling (Majority)** | $\mathbf{u} + \mathbf{v} \Rightarrow [u_i + v_i > 1]$ | Kume olustur |
| **Permutation (Shift)** | $\Pi(\mathbf{u}) \Rightarrow \text{rotate}(\mathbf{u})$ | Sira/rol kodla |

### 22.2 Yapisal Kodlama

Bir gorev kaydi:
$$V_{\text{task}} = (R_{\text{type}} \otimes V_{\text{bug}}) + (R_{\text{project}} \otimes V_{\text{fridex}}) + (R_{\text{status}} \otimes V_{\text{pending}})$$

**Sorgulama (unbinding):**
$$\hat{V}_{\text{project}} = V_{\text{task}} \otimes R_{\text{project}}^{-1}$$
$$\text{nearest}(\hat{V}_{\text{project}}, \text{vocabulary}) = V_{\text{fridex}}$$

**Avantaj:** $O(D)$ islemle herhangi bir alan sorgulanabilir. Geleneksel veritabanina gore cok hizli, hafiza-verimli.

### 22.3 Neden VSA?

- **Hiz:** $O(D)$ operasyonlar, $D = 10K$ ile ~microsaniye
- **Hafiza:** Tek vektor = 10K bit = 1.25KB (vs. 768-float embedding = 3KB)
- **Birlesim:** Farkli tipteki bilgiler tek bir vektorde birlestirilebilir
- **Gurultuye dayaniklilik:** Kismen bozuk vektorler hala sorgulanabilir

> **Uretim notu:** VSA, knowledge graph'in hizli on-bellegi olarak kullanilabilir. Detayli sorgular icin graph DB'ye dusulur.

---

## 23. Embedding ve Benzerlik Matematigi

### 23.1 Sentence Embedding Uretimi

Kullanici sorgulari ve hafiza dugumlerinin vektör temsilini olusturma:

**Mean Pooling:**
$$\mathbf{e}_{\text{sentence}} = \frac{1}{T} \sum_{t=1}^{T} \mathbf{h}_t \odot \mathbf{m}_t$$

- $\mathbf{h}_t$: $t$'inci token'in encoder ciktisi
- $\mathbf{m}_t$: attention mask (padding tokenlerini disla)

### 23.2 Benzerlik Metrikleri

**Cosine Similarity:**
$$\cos(\mathbf{a}, \mathbf{b}) = \frac{\mathbf{a} \cdot \mathbf{b}}{\|\mathbf{a}\| \|\mathbf{b}\|} = \frac{\sum_i a_i b_i}{\sqrt{\sum_i a_i^2} \sqrt{\sum_i b_i^2}}$$

**Euclidean Distance (L2):**
$$d_2(\mathbf{a}, \mathbf{b}) = \sqrt{\sum_i (a_i - b_i)^2}$$

### 23.3 HNSW — Hierarchical Navigable Small World

Vektor veritabaninda yaklasik en yakin komsu aramasi:

- Katmanli graf yapisi: ust katmanlar seyrek (hizli navigasyon), alt katmanlar yogun (hassas arama)
- Arama karmasikligi: $O(\log N)$ (N: toplam vektor sayisi)
- Ekleme karmasikligi: $O(\log N)$

**Parametreler:**
- $M$: dugum basina maksimum kenar sayisi (tipik: 16)
- $\text{ef}$: arama sirasinda incelenen aday sayisi (tipik: 200)

---

## 24. Bilissel Evrim Yol Haritasi

```
Simdi (v0): LLM Wrapper
  └─ Prompt → API → Response
  └─ Tek kanal (metin), tek yonlu, reaktif

6 ay (v1): Akilli Router
  └─ Intent classifier (local) → dogru model/tool secimi
  └─ Basit hafiza (SQLite + embedding)
  └─ %70 sorgu local'de cozulur
  └─ Maliyet %80 duser

12 ay (v2): Algilayan Sistem
  └─ Perception pipeline (ekran, ses, sistem)
  └─ World model (knowledge graph + temporal)
  └─ Proaktif oneriler baslar
  └─ "Seni anlayan" asistan hissi

18 ay (v3): Ogrenen Sistem
  └─ LoRA fine-tuning ile kisisellestirme
  └─ Beceri kazanma (LLM → local distillation)
  └─ Kullanici kalip tanima
  └─ Knowledge graph canli ve zengin

24 ay (v4): Otonom Ajan
  └─ Uzun gorevlerde bagimsiz calisma
  └─ Multi-modal algi + cikti
  └─ Tahmine dayali proaktif aksiyon
  └─ "Gercek JARVIS" hissi
  └─ Duygu + baglam duyarli iletisim
```

### 24.1 Olgunluk Metrikleri

Her evrim asamasi icin olculebilir hedefler:

| Asama | Metrik | Hedef |
|-------|--------|-------|
| v0 | Yanit dogru orani | > %85 |
| v1 | Local cozum orani | > %70 |
| v1 | Ortalama yanit suresi | < 200ms (L0-L1) |
| v2 | Proaktif oneri kabul orani | > %40 |
| v2 | Durum tahmini dogrulugu | > %75 |
| v3 | Aylik iyilestirme | > %10 basari artisi |
| v3 | Distilled model basarisi | > %80 (teacher'a gore) |
| v4 | Otonom gorev tamamlama | > %90 |
| v4 | Kullanici memnuniyeti | > 4.5/5 |

---

## Ek A: Notasyon Rehberi

| Sembol | Anlam |
|--------|-------|
| $\mathbf{x}$ | Girdi vektoru |
| $\mathbf{h}$ | Gizli durum vektoru |
| $\mathbf{W}$ | Agirlik matrisi |
| $\sigma(\cdot)$ | Sigmoid fonksiyonu: $\frac{1}{1+e^{-x}}$ |
| $\text{softmax}(\cdot)$ | Normalize edilmis ustel fonksiyon |
| $\mathrm{D}_{KL}[\cdot \| \cdot]$ | Kullback-Leibler diverjans |
| $\mathcal{L}$ | Kayip (loss) fonksiyonu |
| $\theta$ | Model parametreleri |
| $\alpha, \beta, \gamma, \lambda, \mu$ | Hiperparametreler |
| $\odot$ | Element-wise (Hadamard) carpim |
| $\otimes$ | Binding / dis carpim |
| $\mathcal{N}(v)$ | Dugum $v$'nin komsulari |
| $B_t(s)$ | Zaman $t$'deki inanc durumu |
| $G(\pi)$ | Politika $\pi$'nin beklenen serbest enerjisi |
| $U(a)$ | Eylem $a$'nin faydasi |
| $P(\cdot | \cdot)$ | Kosullu olasilik |

---

## Ek B: Algoritma Karmasikliklari Ozeti

| Algoritma | Zaman | Uzay | Kullanim |
|-----------|-------|------|----------|
| Self-Attention | $O(n^2 d)$ | $O(n^2)$ | Intent classifier, NER |
| CRF Viterbi | $O(T K^2)$ | $O(TK)$ | NER etiket dizisi |
| CTC Forward | $O(T \|\mathbf{y}\|)$ | $O(T \|\mathbf{y}\|)$ | STT hizalama |
| TransE | $O(\|E\| d)$ | $O((\|V\|+\|E\|)d)$ | KG embedding |
| GNN Message Passing | $O(\|E\| d)$ | $O(\|V\| d)$ | Cizge baglam yayilimi |
| HNSW Search | $O(\log N)$ | $O(N M)$ | Vektor benzerlik |
| Baum-Welch | $O(T K^2)$ | $O(TK)$ | HMM parametre ogrenme |
| LoRA Forward | $O(r(d+k))$ | $O(r(d+k))$ | Fine-tuning |
| Bayesian Update | $O(K)$ | $O(K)$ | Inanc guncelleme |
| VSA Binding | $O(D)$ | $O(D)$ | Hiperboyutlu islem |
| Perceptual Hash | $O(N^2 \log N)$ | $O(N^2)$ | Gorsel regresyon |
| LinUCB | $O(d^2 K)$ | $O(Kd^2)$ | Model secimi |

---

## Ek C: Referanslar ve Teorik Kaynaklar

1. **Friston, K.** (2010). "The free-energy principle: a unified brain theory?" — Active Inference temeli
2. **Vaswani, A. et al.** (2017). "Attention Is All You Need" — Transformer mimarisi
3. **Hu, E. et al.** (2021). "LoRA: Low-Rank Adaptation of Large Language Models"
4. **Hinton, G. et al.** (2015). "Distilling the Knowledge in a Neural Network" — Knowledge Distillation
5. **Bordes, A. et al.** (2013). "Translating Embeddings for Modeling Multi-relational Data" — TransE
6. **Velickovic, P. et al.** (2018). "Graph Attention Networks" — GAT
7. **Malkov, Y. & Yashunin, D.** (2018). "Efficient and robust approximate nearest neighbor using HNSW"
8. **Graves, A. et al.** (2006). "Connectionist Temporal Classification" — CTC Loss
9. **Lafferty, J. et al.** (2001). "Conditional Random Fields" — CRF for sequence labeling
10. **Kanerva, P.** (2009). "Hyperdimensional Computing" — VSA/HDC
11. **Li, L. et al.** (2010). "A Contextual-Bandit Approach to Personalized News Article Recommendation" — LinUCB
12. **Rabiner, L.** (1989). "A Tutorial on Hidden Markov Models" — HMM
