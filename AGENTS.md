# Friday Agent Guide

> **Amaç:** Proje hedeflerini, mimari ilkeleri öğretmek; üretimde DRY, KISS, YAGNI + algoritmik mükemmellik uygulat.  
> **Ton:** Resmi, net, pratik. **Polyrepo** yaklaşımı.

---

## 0) Kullanım Talimatı

- **Algoritmik düşünce önce:** Zaman/uzay karmaşıklığını analiz et (Big-O), optimal çözüm öner.
- **Önce düşün:** Kompleks işlerde 6–10 maddelik tasarım + karmaşıklık özeti ver.
- **Minimal diff:** Sadece gerekli satırları değiştir.
- **Self-review:** Her üretim sonunda tek paragraf (lint, edge case, performans, güvenlik, **karmaşıklık analizi**).
- **Güvenlik:** PII maskeleme, log kısıntısı, gizli anahtar yönetimi.
- **En basit OPTIMAL çözüm:** Gereksiz soyutlamadan kaçın ama performansı feda etme.
- **Git:** Her anlamlı değişiklik sonrası otomatik commit; `--no-verify` kullanma ve mesajları daima Türkçe, açıklayıcı yaz.

### ⚠️ YASAKLAR

- ❌ Mock/test data (gerçek DB kullan)
- ❌ Unit test obsesyonu
- ❌ Two-pass yaklaşımı
- ❌ Aşırı test (sadece kritik mantık)
- ❌ Test-first
- ❌ Brute-force çözümler (O(n²) yerine O(n log n) varsa)
- ❌ Gereksiz nested loops
- ❌ Global state mutation
- ❌ Side-effect'li fonksiyonlar (gerekmedikçe)

---

## 1) Proje Bağlamı

- **Hedef:** Multi-tenant SaaS platformu
- **Mimari:** Edge (Cloudflare) → Origin (AWS)
- **Kısıtlar:** P95 <100ms, GDPR uyumlu

---

## 2) İlkeler

- **DRY:** ≥3 tekrar sonrası soyutla
- **KISS:** En basit çalışan OPTIMAL çözüm (basit ≠ yavaş)
- **YAGNI:** Bugün gerekmeyen esneklik ekleme
- **Performans:** P95 bütçesi, bundle boyutu, **algoritmik verimlilik**
- **Güvenlik:** Şema doğrulama, RBAC/ABAC, gizli yönetim
- **Observability:** Log + metric + trace
- **Dayanıklılık:** Timeout, retry, circuit breaker
- **A11y:** WCAG 2.1 AA
- **i18n:** Harici kaynaklar
- **Secrets:** KMS/Secrets Manager

---

## 3) Algoritmik Mükemmellik

### 3.1) Karmaşıklık Analizi (Zorunlu)

Her fonksiyon için:
- **Time complexity:** O(?), worst/average/best case
- **Space complexity:** O(?), auxiliary space
- **Trade-off:** Neden bu çözüm? Alternatifler?

```typescript
// ❌ Kötü: O(n²) nested loop
function findDuplicates(arr: number[]): number[] {
  const dupes: number[] = [];
  for (let i = 0; i < arr.length; i++) {
    for (let j = i + 1; j < arr.length; j++) {
      if (arr[i] === arr[j]) dupes.push(arr[i]);
    }
  }
  return dupes;
}

// ✅ İyi: O(n) hash set
function findDuplicates(arr: number[]): number[] {
  const seen = new Set<number>();
  const dupes = new Set<number>();
  for (const num of arr) {
    if (seen.has(num)) dupes.add(num);
    seen.add(num);
  }
  return Array.from(dupes);
}
// Time: O(n), Space: O(n)
```

### 3.2) Veri Yapısı Seçimi

Doğru veri yapısını seç:
- **Array:** Sıralı erişim, cache locality
- **Set/Map:** O(1) lookup, uniqueness
- **Heap:** Priority queue, O(log n) insert/extract
- **Trie:** Prefix search, O(m) where m = key length
- **Graph:** İlişkisel veri, BFS/DFS

```typescript
// ❌ Kötü: Array'de search O(n)
const users: User[] = [...];
const user = users.find(u => u.id === targetId); // O(n)

// ✅ İyi: Map ile O(1) lookup
const usersById = new Map<string, User>(
  users.map(u => [u.id, u])
);
const user = usersById.get(targetId); // O(1)
```

### 3.3) Fonksiyonel Saflık (Pure Functions)

```typescript
// ❌ Kötü: Side-effect, mutasyon
let total = 0;
function addToTotal(value: number): void {
  total += value; // Global state mutation
}

// ✅ İyi: Pure, immutable
function add(a: number, b: number): number {
  return a + b; // Deterministic, testable
}

// ✅ İyi: Immutable update
function updateUser(user: User, updates: Partial<User>): User {
  return { ...user, ...updates }; // Yeni obje döner
}
```

### 3.4) Erken Çıkış (Early Return)

```typescript
// ❌ Kötü: Nested if
function processOrder(order: Order): Result {
  if (order.isValid) {
    if (order.isPaid) {
      if (order.hasStock) {
        return shipOrder(order);
      } else {
        return { error: 'No stock' };
      }
    } else {
      return { error: 'Not paid' };
    }
  } else {
    return { error: 'Invalid' };
  }
}

// ✅ İyi: Guard clauses
function processOrder(order: Order): Result {
  if (!order.isValid) return { error: 'Invalid' };
  if (!order.isPaid) return { error: 'Not paid' };
  if (!order.hasStock) return { error: 'No stock' };
  
  return shipOrder(order);
}
```

### 3.5) Lazy Evaluation

```typescript
// ❌ Kötü: Tüm liste işlenir
const results = items
  .map(expensiveTransform)    // Hepsi işlenir
  .filter(isValid)
  .slice(0, 10);                // Sadece 10 gerekli

// ✅ İyi: Generator ile lazy
function* processItems(items: Item[]) {
  for (const item of items) {
    const transformed = expensiveTransform(item);
    if (isValid(transformed)) {
      yield transformed;
    }
  }
}

const results = Array.from(
  take(processItems(items), 10)
); // Sadece 10 için işlem yapar
```

### 3.6) Memoization (Gerektiğinde)

```typescript
// ❌ Kötü: Repeated expensive computation
function fibonacci(n: number): number {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2); // O(2^n)
}

// ✅ İyi: Memoized
const fibCache = new Map<number, number>();
function fibonacci(n: number): number {
  if (n <= 1) return n;
  if (fibCache.has(n)) return fibCache.get(n)!;
  
  const result = fibonacci(n - 1) + fibonacci(n - 2);
  fibCache.set(n, result);
  return result; // O(n)
}

// ✅ Daha İyi: Iterative DP
function fibonacci(n: number): number {
  if (n <= 1) return n;
  let prev = 0, curr = 1;
  for (let i = 2; i <= n; i++) {
    [prev, curr] = [curr, prev + curr];
  }
  return curr; // O(n), O(1) space
}
```

### 3.7) Batch Operations

```typescript
// ❌ Kötü: N+1 query
async function getUsersWithOrders(userIds: string[]) {
  const users = [];
  for (const id of userIds) {
    const user = await db.user.findUnique({ where: { id } });
    const orders = await db.order.findMany({ where: { userId: id } });
    users.push({ ...user, orders });
  }
  return users; // N+1 queries
}

// ✅ İyi: Batch query
async function getUsersWithOrders(userIds: string[]) {
  const [users, orders] = await Promise.all([
    db.user.findMany({ where: { id: { in: userIds } } }),
    db.order.findMany({ where: { userId: { in: userIds } } })
  ]);
  
  const ordersMap = orders.reduce((acc, order) => {
    (acc[order.userId] ??= []).push(order);
    return acc;
  }, {} as Record<string, Order[]>);
  
  return users.map(user => ({
    ...user,
    orders: ordersMap[user.id] ?? []
  })); // 2 queries toplam
}
```

### 3.8) Naming Conventions (Algoritmik Netlik)

```typescript
// ❌ Kötü: Belirsiz isimler
function proc(d: any): any {
  const r = [];
  for (let i = 0; i < d.length; i++) {
    if (d[i] > 0) r.push(d[i] * 2);
  }
  return r;
}

// ✅ İyi: Anlamlı, açıklayıcı
function doublePositiveNumbers(numbers: number[]): number[] {
  return numbers
    .filter(n => n > 0)
    .map(n => n * 2);
}
```

### 3.9) Single Responsibility

```typescript
// ❌ Kötü: God function
function processUserData(data: any) {
  // Validation
  if (!data.email) throw new Error('Invalid');
  
  // Transformation
  const user = {
    email: data.email.toLowerCase(),
    name: data.name.trim()
  };
  
  // Business logic
  if (user.email.includes('admin')) {
    user.role = 'admin';
  }
  
  // Persistence
  db.save(user);
  
  // Notification
  sendEmail(user.email, 'Welcome');
  
  return user;
}

// ✅ İyi: Separated concerns
function validateUserData(data: unknown): UserInput {
  const schema = z.object({
    email: z.string().email(),
    name: z.string().min(1)
  });
  return schema.parse(data);
}

function normalizeUser(input: UserInput): User {
  return {
    email: input.email.toLowerCase(),
    name: input.name.trim()
  };
}

function assignRole(user: User): User {
  return {
    ...user,
    role: user.email.includes('admin') ? 'admin' : 'user'
  };
}

async function createUser(data: unknown): Promise<User> {
  const input = validateUserData(data);
  const normalized = normalizeUser(input);
  const withRole = assignRole(normalized);
  
  const saved = await db.user.create(withRole);
  await sendWelcomeEmail(saved.email);
  
  return saved;
}
// Her fonksiyon tek sorumluluk, test edilebilir, compose edilebilir
```

### 3.10) Tip Güvenliği (Algoritma Doğruluğu)

```typescript
// ❌ Kötü: any, runtime hata riski
function merge(a: any, b: any): any {
  return { ...a, ...b };
}

// ✅ İyi: Generic, type-safe
function merge<T extends object, U extends object>(
  a: T,
  b: U
): T & U {
  return { ...a, ...b };
}

// ✅ Daha İyi: Discriminated union
type Result<T, E = Error> =
  | { success: true; data: T }
  | { success: false; error: E };

function divide(a: number, b: number): Result<number> {
  if (b === 0) {
    return { success: false, error: new Error('Division by zero') };
  }
  return { success: true, data: a / b };
}

// Kullanım: Compile-time safe
const result = divide(10, 2);
if (result.success) {
  console.log(result.data); // Type: number
} else {
  console.error(result.error); // Type: Error
}
```

---

## 4) Polyrepo

- Her servis ayrı repo
- Ortak kod küçük paketler
- Template'ler ile tutarlılık
- Bağımsız versiyonlama + CHANGELOG

---

## 5) Multi-Tenant + RBAC/ABAC

- **Tenant izolasyonu:** Postgres RLS (`tenant_id` zorunlu)
- **Context:** `tenant_id`, `subject_id`, `roles`, `attributes` zorunlu
- **RBAC:** Minimal roller, least privilege
- **ABAC:** OPA/Cedar/Casbin policy-as-code
- **Çapraz tenant yasak:** Audit log'da `tenant_id` + `decision_id`
- **Cache:** Key'lere `tenant_id` ön eki

---

## 6) Kod Stili

- **Dil:** TypeScript strict mode
- **Yorumlar:** NEDEN'i açıkla + karmaşıklık analizi
- **Hatalar:** `application/problem+json`
- **API:** Dar yüzey, versiyonlu
- **Bağımlılık:** Version pinning, scan
- **Fonksiyonlar:** Max 20 satır, tek sorumluluk
- **Cyclomatic complexity:** <10
- **Nesting depth:** Max 3 seviye

---

## 7) Docker

- Multi-stage build
- Üretim: distroless/slim, non-root, read-only FS
- HEALTHCHECK, ulimits
- SBOM (Syft) + imza (cosign)
- Trivy tarama, kritik blok
- CPU/mem limit zorunlu

---

## 8) Cloudflare + AWS

**Cloudflare:** WAF, rate limit, Turnstile, cache, Workers  
**AWS:** IAM least privilege, VPC, SSM/Secrets, KMS, CloudTrail  
**Observability:** CloudWatch/X-Ray, correlation ID  
**Storage:** S3 encrypted, versioned  
**Deploy:** Blue/green, auto rollback

---

## 9) Observability

- **Log:** Structured, PII masked, `tenant_id`
- **Metrics:** Error/latency/throughput, tenant + version tags
- **Trace:** External calls, DB, cache
- **Chaos:** Timeout, outage scenarios
- **Performance:** P50/P95/P99, slow query alerts

---

## 10) Test

- **Piramit:** Unit > Integration > E2E
- **Bug → test:** Regresyon zorunlu
- **Contract:** Servisler arası
- **Deterministik:** Flaky yasak
- **Risk odaklı:** Kritik yollar
- **Edge cases:** Null, empty, boundary values
- **Algorithmic:** Karmaşıklık assertion'ları

```typescript
// ✅ Karmaşıklık testi
describe('findDuplicates', () => {
  it('should be O(n) time complexity', () => {
    const sizes = [1000, 2000, 4000, 8000];
    const times: number[] = [];
    
    for (const size of sizes) {
      const arr = Array.from({ length: size }, (_, i) => i % 100);
      const start = performance.now();
      findDuplicates(arr);
      times.push(performance.now() - start);
    }
    
    // Linear growth check (tolerance ±30%)
    const ratio1 = times[1] / times[0];
    const ratio2 = times[2] / times[1];
    expect(ratio2).toBeCloseTo(ratio1, 0.3);
  });
});
```

---

## 11) Stack Guardrails

### TypeScript
- `strict: true`, runtime schema (Zod)
- Discriminated union, exhaustiveness
- **No `any`**, prefer `unknown` + narrowing
- Generics > type assertions

### Node.js (Express/NestJS/Fastify)
- Controller → Service → Repo
- Schema validation, `problem+json`
- Helmet, CORS, rate limit, JWT
- Pino + OpenTelemetry
- **N+1 killer:** DataLoader/batch
- Async/await > callbacks
- Stream for large data

### Python (FastAPI/Django)
- `pyright`/`mypy`, Pydantic v2
- Async DB, dependency injection
- `uv`/`pip-tools`, multi-stage Docker
- List comprehension > loops (when readable)
- Generator for memory efficiency

### Go
- Error wrap `%w`, context zorunlu
- `go mod tidy`, interface tüketici tarafında
- chi/gin, timeouts, pprof
- Goroutine + channel, defer cleanup
- Slice pre-allocation

### Rust
- `clippy` + `rustfmt`, `Result` + `?`
- axum/actix, tower middleware
- `serde`, `sqlx`/`sea-orm`
- Zero-copy where possible
- Iterator chains > loops

---

## 12) DB Guardrails

- Küçük backward-compat migrations
- Constraints, soft-delete policy
- **Index strategy:** Covering index, partial index
- **Query optimization:** EXPLAIN ANALYZE
- N+1 engel, transaction sınırları
- `tenant_id` + RLS, audit
- Connection pooling

---

## 13) Prompt Kısayolları

**Algoritmik çözüm**
```
Problem analizi yap:
1. Input/output tanımla
2. Edge cases listele
3. Brute-force O(?)
4. Optimal çözüm O(?)
5. Trade-off'lar
6. Implementation + karmaşıklık yorumu
```

**Tasarım → Kod**
```
8-10 madde tasarım + karmaşıklık özeti.
Onay sonrası minimal diff + test + self-review.
Karmaşıklık analizi ekle (time/space).
```

**Refactor**
```
Mevcut kod karmaşıklığını analiz et.
Optimize edilebilir kısımları belirt.
Pure functions, immutability, early return uygula.
Cyclomatic complexity <10, nesting <3.
```

**Multi-tenant**
```
tenant_id + subject_id + roles + attributes context.
OPA/Cedar kararı, deny-default, RLS enforcement.
Batch operations for multi-tenant queries.
```

**Performance**
```
Profiling yap (CPU/memory/IO).
Hot paths belirle, optimize et.
Caching strategy (Redis/local).
Database query optimization.
```

---

## 14) Self-Review

1. ✓ **Algorithmic:** Time/space complexity optimal? Trade-offs açık?
2. ✓ **Pure functions:** Side-effect minimal? Immutable data?
3. ✓ **Early return:** Guard clauses? Nesting <3?
4. ✓ **Naming:** Açıklayıcı? Consistent?
5. ✓ **Single responsibility:** Her fonksiyon <20 satır?
6. ✓ **Type safety:** No `any`, generic doğru kullanılmış?
7. ✓ Lint & type-check
8. ✓ Edge cases (null, empty, boundary)
9. ✓ Performance (P95/P99, bundle, query)
10. ✓ Schema validation
11. ✓ Security & PII (masking, secrets, CSP)
12. ✓ Multi-tenant (`tenant_id`, RLS, batch)
13. ✓ RBAC/ABAC (policy test, deny-default)
14. ✓ Observability (log/metric/trace)
15. ✓ Tests (unit + edge cases + complexity)
16. ✓ Docker (size, non-root, SBOM, scan)
17. ✓ Deploy (canary, rollback)
18. ✓ Docs (README, CHANGELOG, complexity notes)
19. ✓ Minimal diff

---

## 15) Bakım

- **Polyrepo:** SemVer, auto CHANGELOG
- **Template:** CI, linter, Dockerfile sync
- **Dependencies:** Renovate/Dependabot, auto PR
- **Deprecation:** Bildirim, geçiş, plan
- **Policy sync:** CI ile dağıt
- **Postmortem:** Eylem takip, backlog
- **Performance audit:** Quarterly profiling
- **Code review:** Algorithmic complexity check
- **Audit:** 3 ayda gözden geçir, yılda revizyon

--- project-doc ---

## Fridex Agent Guide

All docs must canonical, no past commentary, only live state.

## Agent Memory (Project Scratchpad)

Purpose: keep lightweight, durable project memory so agents avoid repeating mistakes and follow user/project preferences over time.

### Memory Location (Repo Root)

Store memory in the project root under `./memory/`:

- `memory/decisions.md` — durable architecture/implementation decisions and conventions
- `memory/mistakes.md` — mistakes, fixes, and prevention rules
- `memory/todo.md` — open loops and follow-up tasks
- `memory/context.md` — optional short-lived working context (can be compacted)

### Automatic Write Rules

Agents should append an entry when ANY of the following happens:

1. User states a stable preference or rule ("do it this way").
2. Agent makes a non-trivial mistake and corrects it.
3. A decision is made that affects future implementation.
4. A follow-up task is identified but not completed immediately.

Do NOT write:
- trivial chatter
- transient debug noise
- secrets/tokens/passwords
- private data not required for project execution

### Required Read Rules (Before Work)

Before starting a task, agents must read:

1. `memory/decisions.md`
2. recent entries in `memory/mistakes.md`
3. open items in `memory/todo.md`

Then apply those constraints during planning and implementation.

### Entry Format (Append-only)

Use this compact format:

```md
## YYYY-MM-DD HH:mm
Context: <task or feature>
Type: decision | mistake | preference | todo
Event: <what happened>
Action: <what changed / fix applied>
Rule: <one-line future behavior>
```

### Mistake Entry Requirements

For entries in `memory/mistakes.md`, include:

- `Root cause:`
- `Fix applied:`
- `Prevention rule:`

### Maintenance

- Keep memory append-only by default.
- Compaction is allowed into summaries, but do not silently remove meaning.
- Preserve recent detail (at least latest 30 days) before aggressive compaction.

## Project Summary
Fridex is a macOS Tauri app that orchestrates Codex agents across local workspaces. The frontend is React + Vite; the backend is a Tauri Rust process that spawns `codex app-server` per workspace and streams JSON-RPC events.

- Frontend: React + Vite
- Backend (app): Tauri Rust process
- Backend (daemon): `src-tauri/src/bin/codex_monitor_daemon.rs`
- Shared backend domain logic: `src-tauri/src/shared/*`

## Backend Architecture

The backend separates shared domain logic from environment wiring.

- Shared domain/core logic: `src-tauri/src/shared/*`
- App wiring and platform concerns: feature folders + adapters
- Daemon wiring and transport concerns: `src-tauri/src/bin/codex_monitor_daemon.rs`

## Feature Folders

### Codex

- `src-tauri/src/codex/mod.rs`
- `src-tauri/src/codex/args.rs`
- `src-tauri/src/codex/home.rs`
- `src-tauri/src/codex/config.rs`

### Files

- `src-tauri/src/files/mod.rs`
- `src-tauri/src/files/io.rs`
- `src-tauri/src/files/ops.rs`
- `src-tauri/src/files/policy.rs`

### Dictation

- `src-tauri/src/dictation/mod.rs`
- `src-tauri/src/dictation/real.rs`
- `src-tauri/src/dictation/stub.rs`

### Workspaces

- `src-tauri/src/workspaces/*`

### Shared Core Layer

- `src-tauri/src/shared/*`

Root-level single-file features remain at `src-tauri/src/*.rs` (for example: `menu.rs`, `prompts.rs`, `terminal.rs`, `remote_backend.rs`).

## Shared Core Modules (Source of Truth)

Shared logic that must work in both the app and the daemon lives under `src-tauri/src/shared/`.

- `src-tauri/src/shared/codex_core.rs`
  - Threads, approvals, login/cancel, account, skills, config model
- `src-tauri/src/shared/workspaces_core.rs`
  - Workspace/worktree operations, persistence, sorting, git command helpers
- `src-tauri/src/shared/settings_core.rs`
  - App settings load/update, Codex config path
- `src-tauri/src/shared/files_core.rs`
  - File read/write logic
- `src-tauri/src/shared/git_core.rs`
  - Git command helpers and remote/branch logic
- `src-tauri/src/shared/worktree_core.rs`
  - Worktree naming/path helpers and clone destination helpers
- `src-tauri/src/shared/account.rs`
  - Account helper utilities and tests

## App/Daemon Pattern

Use this mental model when changing backend code:

1. Put shared logic in a shared core module.
2. Keep app and daemon code as thin adapters.
3. Pass environment-specific behavior via closures or small adapter helpers.

The app and daemon do not re-implement domain logic.

## Daemon Module Wrappers

The daemon defines wrapper modules named `codex` and `files` inside `src-tauri/src/bin/codex_monitor_daemon.rs`.

These wrappers re-export the daemon’s local modules:

- Codex: `codex_args`, `codex_home`, `codex_config`
- Files: `file_io`, `file_ops`, `file_policy`

Shared cores use `crate::codex::*` and `crate::files::*` paths. The daemon wrappers satisfy those paths without importing app-only modules.

## Key Paths

### Frontend

- Composition root: `src/App.tsx`
- Feature slices: `src/features/`
- Tauri IPC wrapper: `src/services/tauri.ts`
- Tauri event hub: `src/services/events.ts`
- Shared UI types: `src/types.ts`
- Thread item normalization: `src/utils/threadItems.ts`
- Styles: `src/styles/`

### Backend (App)

- Tauri command registry: `src-tauri/src/lib.rs`
- Codex adapters: `src-tauri/src/codex/*`
- Files adapters: `src-tauri/src/files/*`
- Dictation adapters: `src-tauri/src/dictation/*`
- Workspaces adapters: `src-tauri/src/workspaces/*`
- Shared core layer: `src-tauri/src/shared/*`
- Git feature: `src-tauri/src/git/mod.rs`

### Backend (Daemon)

- Daemon entrypoint: `src-tauri/src/bin/codex_monitor_daemon.rs`
- Daemon imports shared cores via `#[path = "../shared/mod.rs"] mod shared;`

## Architecture Guidelines

### Frontend Guidelines

- Composition root: keep orchestration in `src/App.tsx`.
- Components: presentational only. Props in, UI out. No Tauri IPC.
- Hooks: own state, side effects, and event wiring.
- Utils: pure helpers only in `src/utils/`.
- Services: all Tauri IPC goes through `src/services/`.
- Types: shared UI types live in `src/types.ts`.
- Styles: one CSS file per UI area under `src/styles/`.

Keep `src/App.tsx` lean:

- Keep it to wiring: hook composition, layout, and assembly.
- Move stateful logic/effects into hooks under `src/features/app/hooks/`.
- Keep Tauri IPC, menu listeners, and subscriptions out of `src/App.tsx`.

### Design System Usage

Use the design-system layer for shared UI shells and tokenized styling.

- Primitive component locations:
  - `src/features/design-system/components/modal/ModalShell.tsx`
  - `src/features/design-system/components/toast/ToastPrimitives.tsx`
  - `src/features/design-system/components/panel/PanelPrimitives.tsx`
  - `src/features/design-system/components/popover/PopoverPrimitives.tsx`
  - Toast sub-primitives: `ToastHeader`, `ToastActions`, `ToastError` (in `ToastPrimitives.tsx`)
  - Panel sub-primitives: `PanelMeta`, `PanelSearchField` (in `PanelPrimitives.tsx`)
  - Popover sub-primitives: `PopoverMenuItem` (in `PopoverPrimitives.tsx`)
- Diff theming and style bridge:
  - `src/features/design-system/diff/diffViewerTheme.ts`
- DS token/style locations:
  - `src/styles/ds-tokens.css`
  - `src/styles/ds-modal.css`
  - `src/styles/ds-toast.css`
  - `src/styles/ds-panel.css`
  - `src/styles/ds-popover.css`
  - `src/styles/ds-diff.css`

Naming conventions:

- DS CSS classes use `.ds-*` prefixes.
- DS CSS variables use `--ds-*` prefixes.
- DS React primitives use `PascalCase` component names (`ModalShell`, `ToastCard`, `ToastHeader`, `ToastActions`, `ToastError`, `PanelFrame`, `PanelHeader`, `PanelMeta`, `PanelSearchField`, `PopoverSurface`, `PopoverMenuItem`).
- Feature CSS should keep feature-prefixed classes (`.worktree-*`, `.update-*`) for content/layout specifics.

Do:

- Use DS primitives first for shared shells (modal wrappers, toast cards/viewports, panel shells/headers, popover/dropdown surfaces).
- Pull shared visual tokens from `--ds-*` variables.
- Keep feature styles focused on feature-specific layout/content, not duplicated shell chrome.
- Centralize shared animation/chrome in DS stylesheets when used by multiple feature families.

Don't:

- Recreate fixed modal backdrops/cards in feature CSS when `ModalShell` is used.
- Duplicate toast card chrome (background/border/shadow/padding/enter animation) per toast family.
- Duplicate panel shell layout/header alignment in feature styles when `PanelFrame`/`PanelHeader` already provide it.
- Recreate popover/dropdown shell chrome in feature CSS when `PopoverSurface`/`PopoverMenuItem` already provide it.
- Add new non-DS color constants for shared shells; add/extend DS tokens instead.

Migration guidance for new/updated components:

1. Start by wrapping UI in the closest DS primitive.
2. Migrate shared shell styles into DS CSS (`ds-*.css`) and delete redundant feature-level shell selectors.
3. Keep only feature-local classes for spacing/content/interaction details.
4. For legacy selectors that are still referenced, keep minimal compatibility aliases temporarily.
5. Remove compatibility aliases once callsites reach zero, then rerun lint/typecheck/tests.

Anti-duplication guidance:

- Before adding shell styles, search for existing DS token/primitive coverage.
- If two or more feature files need the same shell rule, move it to DS CSS immediately.
- Prefer extending DS primitives/tokens over introducing another feature-specific wrapper class.
- During refactors, remove unused legacy selectors once callsites are migrated.

Enforcement workflow:

- Lint guardrails for DS-targeted files live in `.eslintrc.cjs`.
- Popover guardrails are enforced for migrated popover files (`MainHeader`, `Sidebar`, `SidebarHeader`, `SidebarCornerActions`, `OpenAppMenu`, `LaunchScript*`, `ComposerInput`, `FilePreviewPopover`, `WorkspaceHome`) to require `PopoverSurface`/`PopoverMenuItem`.
- Codemod scripts live in `scripts/codemods/`:
  - `modal-shell-codemod.mjs`
  - `panel-shell-codemod.mjs`
  - `toast-shell-codemod.mjs`
- Run `npm run codemod:ds:dry` before UI shell migration PRs.
- Keep `npm run lint:ds`/`npm run lint` green for modal/toast/panel/popover/diff files.

### Backend Guidelines

- Shared logic goes in `src-tauri/src/shared/` first.
- App and daemon are thin adapters around shared cores.
- Avoid duplicating git/worktree/codex/settings/files logic in adapters.
- Prefer explicit, readable adapter helpers over clever abstractions.
- Do not folderize single-file features unless you are splitting them.

## Daemon: How and When to Add Code

The daemon runs backend logic outside the Tauri app.

### When to Update the Daemon

Update the daemon when one of these is true:

- A Tauri command is used in remote mode.
- The daemon exposes the same behavior over its JSON-RPC transport.
- Shared core behavior changes and the daemon wiring must pass new inputs.

### Where Code Goes

1. Shared behavior or domain logic:
   - Add or update code in `src-tauri/src/shared/*.rs`.
2. App-only behavior:
   - Update the app adapters or Tauri commands.
3. Daemon-only transport/wiring behavior:
   - Update `src-tauri/src/bin/codex_monitor_daemon.rs`.

### How to Add a New Backend Command

1. Implement the core logic in a shared module.
2. Wire it in the app.
   - Add a Tauri command in `src-tauri/src/lib.rs`.
   - Call the shared core from the appropriate adapter.
   - Mirror it in `src/services/tauri.ts`.
3. Wire it in the daemon.
   - Add a daemon method that calls the same shared core.
   - Add the JSON-RPC handler branch in `codex_monitor_daemon.rs`.

### Adapter Patterns to Reuse

- Shared git unit wrapper:
  - `workspaces_core::run_git_command_unit(...)`
- App spawn adapter:
  - `spawn_with_app(...)` in `src-tauri/src/workspaces/commands.rs`
- Daemon spawn adapter:
  - `spawn_with_client(...)` in `src-tauri/src/bin/codex_monitor_daemon.rs`
- Daemon wrapper modules:
  - `mod codex { ... }` and `mod files { ... }` in `codex_monitor_daemon.rs`

If you find yourself copying logic between app and daemon, extract it into `src-tauri/src/shared/`.

## App-Server Flow

- Backend spawns `codex app-server` using the `codex` binary.
- Initialize with `initialize` and then `initialized`.
- Do not send requests before initialization.
- JSON-RPC notifications stream over stdout.
- Threads are listed via `thread/list` and resumed via `thread/resume`.
- Archiving uses `thread/archive`.

## Event Stack (Tauri → React)

The app uses a shared event hub so each native event has one `listen` and many subscribers.

- Backend emits: `src-tauri/src/lib.rs` emits events to the main window.
- Frontend hub: `src/services/events.ts` defines `createEventHub` and module-level hubs.
- React subscription: use `useTauriEvent(subscribeX, handler)`.

### Adding a New Tauri Event

1. Emit the event in `src-tauri/src/lib.rs`.
2. Add a hub and `subscribeX` helper in `src/services/events.ts`.
3. Subscribe via `useTauriEvent` in a hook or component.
4. Update `src/services/events.test.ts` if you add new subscription helpers.

## Workspace Persistence

- Workspaces live in `workspaces.json` under the app data directory.
- Settings live in `settings.json` under the app data directory.
- On launch, the app connects each workspace once and loads its thread list.

## Common Changes (Where to Look First)

- UI layout or styling:
  - `src/features/*/components/*` and `src/styles/*`
- App-server events:
  - `src/features/app/hooks/useAppServerEvents.ts`
- Tauri IPC shape:
  - `src/services/tauri.ts` and `src-tauri/src/lib.rs`
- Shared backend behavior:
  - `src-tauri/src/shared/*`
- Workspaces/worktrees:
  - Shared core: `src-tauri/src/shared/workspaces_core.rs`
  - App adapters: `src-tauri/src/workspaces/*`
  - Daemon wiring: `src-tauri/src/bin/codex_monitor_daemon.rs`
- Settings and Codex config:
  - Shared core: `src-tauri/src/shared/settings_core.rs`
  - App adapters: `src-tauri/src/codex/config.rs`, `src-tauri/src/settings/mod.rs`
  - Daemon wiring: `src-tauri/src/bin/codex_monitor_daemon.rs`
- Files:
  - Shared core: `src-tauri/src/shared/files_core.rs`
  - App adapters: `src-tauri/src/files/*`
- Codex threads/approvals/login:
  - Shared core: `src-tauri/src/shared/codex_core.rs`
  - App adapters: `src-tauri/src/codex/*`
  - Daemon wiring: `src-tauri/src/bin/codex_monitor_daemon.rs`

## Threads Feature Split (Frontend)

`useThreads` is a composition layer that wires focused hooks and shared utilities.

- Orchestration: `src/features/threads/hooks/useThreads.ts`
- Actions: `src/features/threads/hooks/useThreadActions.ts`
- Approvals: `src/features/threads/hooks/useThreadApprovals.ts`
- Event handlers: `src/features/threads/hooks/useThreadEventHandlers.ts`
- Messaging: `src/features/threads/hooks/useThreadMessaging.ts`
- Storage: `src/features/threads/hooks/useThreadStorage.ts`
- Status helpers: `src/features/threads/hooks/useThreadStatus.ts`
- Selectors: `src/features/threads/hooks/useThreadSelectors.ts`
- Rate limits: `src/features/threads/hooks/useThreadRateLimits.ts`
- Collab links: `src/features/threads/hooks/useThreadLinking.ts`

## Running Locally

```bash
npm install
npm run tauri dev
```

## Release Build

```bash
npm run tauri build
```

## Type Checking

```bash
npm run typecheck
```

## Tests

```bash
npm run test
```

```bash
npm run test:watch
```

## Validation

At the end of a task:

1. Run `npm run lint`.
2. Run `npm run test` when you touched threads, settings, updater, shared utils, or backend cores.
3. Run `npm run typecheck`.
4. If you changed Rust backend code, run `cargo check` in `src-tauri`.

## Notes

- The window uses `titleBarStyle: "Overlay"` and macOS private APIs for transparency.
- Avoid breaking JSON-RPC format; the app-server is strict.
- App settings and Codex feature toggles are best-effort synced to `CODEX_HOME/config.toml`.
- UI preferences live in `localStorage`.
- GitHub issues require `gh` to be installed and authenticated.
- Custom prompts are loaded from `$CODEX_HOME/prompts` (or `~/.codex/prompts`).

## Error Toasts

- Use `pushErrorToast` from `src/services/toasts.ts` for user-facing errors.
- Toast wiring:
  - Hook: `src/features/notifications/hooks/useErrorToasts.ts`
  - UI: `src/features/notifications/components/ErrorToasts.tsx`
  - Styles: `src/styles/error-toasts.css`
