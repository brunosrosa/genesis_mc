# Silicon Shock Laboratory — Original Payload (Blob)

## System Prompt (usado em ambas as rotas)

```
Resuma os fatos crus, extraia a alma matemática,
não emita opiniões, limite-se a ~3000 tokens.
```

---

# Silicon Shock Validation Report — 25k Token Dense Payload

## Finding Block 000

severity: HIGH | tool: semgrep | id: SEC-0000-RUST-001
file: src/services/user_service_00.rs | line: 100 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 100, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 70% | CWE-476 | OWASP-A1 | EPSS: 0.1000

code_snippet: | let user = ctx.users_00.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 001

severity: HIGH | tool: semgrep | id: SEC-0001-RUST-001
file: src/services/user_service_01.rs | line: 117 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 117, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 71% | CWE-476 | OWASP-A1 | EPSS: 0.1070

code_snippet: | let user = ctx.users_01.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 002

severity: HIGH | tool: semgrep | id: SEC-0002-RUST-001
file: src/services/user_service_02.rs | line: 134 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 134, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 72% | CWE-476 | OWASP-A1 | EPSS: 0.1140

code_snippet: | let user = ctx.users_02.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 003

severity: HIGH | tool: semgrep | id: SEC-0003-RUST-001
file: src/services/user_service_03.rs | line: 151 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 151, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 73% | CWE-476 | OWASP-A1 | EPSS: 0.1210

code_snippet: | let user = ctx.users_03.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 004

severity: HIGH | tool: semgrep | id: SEC-0004-RUST-001
file: src/services/user_service_04.rs | line: 168 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 168, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 74% | CWE-476 | OWASP-A1 | EPSS: 0.1280

code_snippet: | let user = ctx.users_04.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 005

severity: HIGH | tool: semgrep | id: SEC-0005-RUST-001
file: src/services/user_service_05.rs | line: 185 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 185, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 75% | CWE-476 | OWASP-A1 | EPSS: 0.1350

code_snippet: | let user = ctx.users_05.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 006

severity: HIGH | tool: semgrep | id: SEC-0006-RUST-001
file: src/services/user_service_06.rs | line: 202 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 202, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 76% | CWE-476 | OWASP-A1 | EPSS: 0.1420

code_snippet: | let user = ctx.users_06.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 007

severity: HIGH | tool: semgrep | id: SEC-0007-RUST-001
file: src/services/user_service_07.rs | line: 219 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 219, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 77% | CWE-476 | OWASP-A1 | EPSS: 0.1490

code_snippet: | let user = ctx.users_07.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 008

severity: HIGH | tool: semgrep | id: SEC-0008-RUST-001
file: src/services/user_service_08.rs | line: 236 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 236, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 78% | CWE-476 | OWASP-A1 | EPSS: 0.1560

code_snippet: | let user = ctx.users_08.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 009

severity: HIGH | tool: semgrep | id: SEC-0009-RUST-001
file: src/services/user_service_09.rs | line: 253 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 253, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 79% | CWE-476 | OWASP-A1 | EPSS: 0.1630

code_snippet: | let user = ctx.users_09.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 010

severity: HIGH | tool: semgrep | id: SEC-0010-RUST-001
file: src/services/user_service_10.rs | line: 270 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 270, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 80% | CWE-476 | OWASP-A1 | EPSS: 0.1700

code_snippet: | let user = ctx.users_10.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 011

severity: HIGH | tool: semgrep | id: SEC-0011-RUST-001
file: src/services/user_service_11.rs | line: 287 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 287, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 81% | CWE-476 | OWASP-A1 | EPSS: 0.1770

code_snippet: | let user = ctx.users_11.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 012

severity: HIGH | tool: semgrep | id: SEC-0012-RUST-001
file: src/services/user_service_12.rs | line: 304 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 304, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 82% | CWE-476 | OWASP-A1 | EPSS: 0.1840

code_snippet: | let user = ctx.users_12.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 013

severity: HIGH | tool: semgrep | id: SEC-0013-RUST-001
file: src/services/user_service_13.rs | line: 321 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 321, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 83% | CWE-476 | OWASP-A1 | EPSS: 0.1910

code_snippet: | let user = ctx.users_13.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 014

severity: HIGH | tool: semgrep | id: SEC-0014-RUST-001
file: src/services/user_service_14.rs | line: 338 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 338, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 84% | CWE-476 | OWASP-A1 | EPSS: 0.1980

code_snippet: | let user = ctx.users_14.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 015

severity: HIGH | tool: semgrep | id: SEC-0015-RUST-001
file: src/services/user_service_15.rs | line: 355 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 355, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 85% | CWE-476 | OWASP-A1 | EPSS: 0.2050

code_snippet: | let user = ctx.users_15.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 016

severity: HIGH | tool: semgrep | id: SEC-0016-RUST-001
file: src/services/user_service_16.rs | line: 372 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 372, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 86% | CWE-476 | OWASP-A1 | EPSS: 0.2120

code_snippet: | let user = ctx.users_16.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 017

severity: HIGH | tool: semgrep | id: SEC-0017-RUST-001
file: src/services/user_service_17.rs | line: 389 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 389, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 87% | CWE-476 | OWASP-A1 | EPSS: 0.2190

code_snippet: | let user = ctx.users_17.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 018

severity: HIGH | tool: semgrep | id: SEC-0018-RUST-001
file: src/services/user_service_18.rs | line: 406 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 406, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 88% | CWE-476 | OWASP-A1 | EPSS: 0.2260

code_snippet: | let user = ctx.users_18.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 019

severity: HIGH | tool: semgrep | id: SEC-0019-RUST-001
file: src/services/user_service_19.rs | line: 423 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 423, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 89% | CWE-476 | OWASP-A1 | EPSS: 0.2330

code_snippet: | let user = ctx.users_19.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 020

severity: HIGH | tool: semgrep | id: SEC-0020-RUST-001
file: src/services/user_service_00.rs | line: 440 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 440, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 90% | CWE-476 | OWASP-A1 | EPSS: 0.2400

code_snippet: | let user = ctx.users_00.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 021

severity: HIGH | tool: semgrep | id: SEC-0021-RUST-001
file: src/services/user_service_01.rs | line: 457 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 457, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 91% | CWE-476 | OWASP-A1 | EPSS: 0.2470

code_snippet: | let user = ctx.users_01.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 022

severity: HIGH | tool: semgrep | id: SEC-0022-RUST-001
file: src/services/user_service_02.rs | line: 474 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 474, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 92% | CWE-476 | OWASP-A1 | EPSS: 0.2540

code_snippet: | let user = ctx.users_02.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 023

severity: HIGH | tool: semgrep | id: SEC-0023-RUST-001
file: src/services/user_service_03.rs | line: 491 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 491, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 93% | CWE-476 | OWASP-A1 | EPSS: 0.2610

code_snippet: | let user = ctx.users_03.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 024

severity: HIGH | tool: semgrep | id: SEC-0024-RUST-001
file: src/services/user_service_04.rs | line: 508 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 508, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 94% | CWE-476 | OWASP-A1 | EPSS: 0.2680

code_snippet: | let user = ctx.users_04.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 025

severity: HIGH | tool: semgrep | id: SEC-0025-RUST-001
file: src/services/user_service_05.rs | line: 525 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 525, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 95% | CWE-476 | OWASP-A1 | EPSS: 0.2750

code_snippet: | let user = ctx.users_05.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 026

severity: HIGH | tool: semgrep | id: SEC-0026-RUST-001
file: src/services/user_service_06.rs | line: 542 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 542, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 96% | CWE-476 | OWASP-A1 | EPSS: 0.2820

code_snippet: | let user = ctx.users_06.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 027

severity: HIGH | tool: semgrep | id: SEC-0027-RUST-001
file: src/services/user_service_07.rs | line: 559 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 559, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 97% | CWE-476 | OWASP-A1 | EPSS: 0.2890

code_snippet: | let user = ctx.users_07.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 028

severity: HIGH | tool: semgrep | id: SEC-0028-RUST-001
file: src/services/user_service_08.rs | line: 576 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 576, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 98% | CWE-476 | OWASP-A1 | EPSS: 0.2960

code_snippet: | let user = ctx.users_08.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 029

severity: HIGH | tool: semgrep | id: SEC-0029-RUST-001
file: src/services/user_service_09.rs | line: 593 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 593, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 99% | CWE-476 | OWASP-A1 | EPSS: 0.3030

code_snippet: | let user = ctx.users_09.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 030

severity: HIGH | tool: semgrep | id: SEC-0030-RUST-001
file: src/services/user_service_10.rs | line: 610 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 610, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 70% | CWE-476 | OWASP-A1 | EPSS: 0.3100

code_snippet: | let user = ctx.users_10.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 031

severity: HIGH | tool: semgrep | id: SEC-0031-RUST-001
file: src/services/user_service_11.rs | line: 627 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 627, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 71% | CWE-476 | OWASP-A1 | EPSS: 0.3170

code_snippet: | let user = ctx.users_11.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 032

severity: HIGH | tool: semgrep | id: SEC-0032-RUST-001
file: src/services/user_service_12.rs | line: 644 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 644, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 72% | CWE-476 | OWASP-A1 | EPSS: 0.3240

code_snippet: | let user = ctx.users_12.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 033

severity: HIGH | tool: semgrep | id: SEC-0033-RUST-001
file: src/services/user_service_13.rs | line: 661 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 661, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 73% | CWE-476 | OWASP-A1 | EPSS: 0.3310

code_snippet: | let user = ctx.users_13.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 034

severity: HIGH | tool: semgrep | id: SEC-0034-RUST-001
file: src/services/user_service_14.rs | line: 678 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 678, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 74% | CWE-476 | OWASP-A1 | EPSS: 0.3380

code_snippet: | let user = ctx.users_14.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 035

severity: HIGH | tool: semgrep | id: SEC-0035-RUST-001
file: src/services/user_service_15.rs | line: 695 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 695, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 75% | CWE-476 | OWASP-A1 | EPSS: 0.3450

code_snippet: | let user = ctx.users_15.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 036

severity: HIGH | tool: semgrep | id: SEC-0036-RUST-001
file: src/services/user_service_16.rs | line: 712 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 712, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 76% | CWE-476 | OWASP-A1 | EPSS: 0.3520

code_snippet: | let user = ctx.users_16.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 037

severity: HIGH | tool: semgrep | id: SEC-0037-RUST-001
file: src/services/user_service_17.rs | line: 729 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 729, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 77% | CWE-476 | OWASP-A1 | EPSS: 0.3590

code_snippet: | let user = ctx.users_17.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 038

severity: HIGH | tool: semgrep | id: SEC-0038-RUST-001
file: src/services/user_service_18.rs | line: 746 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 746, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 78% | CWE-476 | OWASP-A1 | EPSS: 0.3660

code_snippet: | let user = ctx.users_18.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 039

severity: HIGH | tool: semgrep | id: SEC-0039-RUST-001
file: src/services/user_service_19.rs | line: 763 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 763, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 79% | CWE-476 | OWASP-A1 | EPSS: 0.3730

code_snippet: | let user = ctx.users_19.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 040

severity: HIGH | tool: semgrep | id: SEC-0040-RUST-001
file: src/services/user_service_00.rs | line: 780 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 780, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 80% | CWE-476 | OWASP-A1 | EPSS: 0.3800

code_snippet: | let user = ctx.users_00.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 041

severity: HIGH | tool: semgrep | id: SEC-0041-RUST-001
file: src/services/user_service_01.rs | line: 797 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 797, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 81% | CWE-476 | OWASP-A1 | EPSS: 0.3870

code_snippet: | let user = ctx.users_01.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 042

severity: HIGH | tool: semgrep | id: SEC-0042-RUST-001
file: src/services/user_service_02.rs | line: 814 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 814, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 82% | CWE-476 | OWASP-A1 | EPSS: 0.3940

code_snippet: | let user = ctx.users_02.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 043

severity: HIGH | tool: semgrep | id: SEC-0043-RUST-001
file: src/services/user_service_03.rs | line: 831 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 831, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 83% | CWE-476 | OWASP-A1 | EPSS: 0.4010

code_snippet: | let user = ctx.users_03.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 044

severity: HIGH | tool: semgrep | id: SEC-0044-RUST-001
file: src/services/user_service_04.rs | line: 848 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 848, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 84% | CWE-476 | OWASP-A1 | EPSS: 0.4080

code_snippet: | let user = ctx.users_04.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 045

severity: HIGH | tool: semgrep | id: SEC-0045-RUST-001
file: src/services/user_service_05.rs | line: 865 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 865, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 85% | CWE-476 | OWASP-A1 | EPSS: 0.4150

code_snippet: | let user = ctx.users_05.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 046

severity: HIGH | tool: semgrep | id: SEC-0046-RUST-001
file: src/services/user_service_06.rs | line: 882 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 882, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 86% | CWE-476 | OWASP-A1 | EPSS: 0.4220

code_snippet: | let user = ctx.users_06.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 047

severity: HIGH | tool: semgrep | id: SEC-0047-RUST-001
file: src/services/user_service_07.rs | line: 899 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 899, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 87% | CWE-476 | OWASP-A1 | EPSS: 0.4290

code_snippet: | let user = ctx.users_07.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 048

severity: HIGH | tool: semgrep | id: SEC-0048-RUST-001
file: src/services/user_service_08.rs | line: 916 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 916, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 88% | CWE-476 | OWASP-A1 | EPSS: 0.4360

code_snippet: | let user = ctx.users_08.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Finding Block 049

severity: HIGH | tool: semgrep | id: SEC-0049-RUST-001
file: src/services/user_service_09.rs | line: 933 | col: 23
pattern: potential_null_dereference -> matches!(ctx, Some(x)) || matches!(ctx, None)
message: In branch at line 933, `ctx` is matched against `Some` and `None` without subsequent null-check on `x` within the same arm. This may cause a null pointer dereference if `x` is accessed before the match completes.
confidence: 89% | CWE-476 | OWASP-A1 | EPSS: 0.4430

code_snippet: | let user = ctx.users_09.get(id)?; let profile = user.profile.as_ref()?;

remediation:
  - Implement comprehensive null checks before accessing optional fields
  - Use if-let chains for cleaner Optional handling
  - Add integration tests covering None paths

参考资料: CWE-476 (NULL Pointer Dereference), RFC-0042 §3.2

---

## Lint Info 000: cargo_clippy | src/harvester/mod.rs @ 50:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 001: cargo_clippy | src/harvester/mod.rs @ 57:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 002: cargo_clippy | src/harvester/mod.rs @ 64:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 003: cargo_clippy | src/harvester/mod.rs @ 71:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 004: cargo_clippy | src/harvester/mod.rs @ 78:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 005: cargo_clippy | src/harvester/mod.rs @ 85:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 006: cargo_clippy | src/harvester/mod.rs @ 92:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 007: cargo_clippy | src/harvester/mod.rs @ 99:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 008: cargo_clippy | src/harvester/mod.rs @ 106:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 009: cargo_clippy | src/harvester/mod.rs @ 113:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 010: cargo_clippy | src/harvester/mod.rs @ 120:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 011: cargo_clippy | src/harvester/mod.rs @ 127:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 012: cargo_clippy | src/harvester/mod.rs @ 134:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 013: cargo_clippy | src/harvester/mod.rs @ 141:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 014: cargo_clippy | src/harvester/mod.rs @ 148:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 015: cargo_clippy | src/harvester/mod.rs @ 155:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 016: cargo_clippy | src/harvester/mod.rs @ 162:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 017: cargo_clippy | src/harvester/mod.rs @ 169:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 018: cargo_clippy | src/harvester/mod.rs @ 176:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 019: cargo_clippy | src/harvester/mod.rs @ 183:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 020: cargo_clippy | src/harvester/mod.rs @ 190:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 021: cargo_clippy | src/harvester/mod.rs @ 197:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 022: cargo_clippy | src/harvester/mod.rs @ 204:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 023: cargo_clippy | src/harvester/mod.rs @ 211:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 024: cargo_clippy | src/harvester/mod.rs @ 218:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 025: cargo_clippy | src/harvester/mod.rs @ 225:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 026: cargo_clippy | src/harvester/mod.rs @ 232:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 027: cargo_clippy | src/harvester/mod.rs @ 239:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 028: cargo_clippy | src/harvester/mod.rs @ 246:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Lint Info 029: cargo_clippy | src/harvester/mod.rs @ 253:10

warning: clippy::unwrap_used — Calls to `.unwrap()` that panick on None/Err.
suggestion: Use `.unwrap_or_else(|| ...)`, `.expect("context")` with message, or restructure with `if let Some(x) = ...`.

---

## Dependency Advisory 000: GHSA-1000-2000-3000 | severity: CRITICAL

package: serde@1.0:0.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_00 -> serde
fixed_version: 1.0.52 | EPSS: 0.3000

---

## Dependency Advisory 001: GHSA-1001-2001-3001 | severity: CRITICAL

package: serde@1.1:3.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_01 -> serde
fixed_version: 1.0.52 | EPSS: 0.3100

---

## Dependency Advisory 002: GHSA-1002-2002-3002 | severity: CRITICAL

package: serde@1.2:6.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_02 -> serde
fixed_version: 1.0.52 | EPSS: 0.3200

---

## Dependency Advisory 003: GHSA-1003-2003-3003 | severity: CRITICAL

package: serde@1.3:9.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_03 -> serde
fixed_version: 1.0.52 | EPSS: 0.3300

---

## Dependency Advisory 004: GHSA-1004-2004-3004 | severity: CRITICAL

package: serde@1.4:2.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_04 -> serde
fixed_version: 1.0.52 | EPSS: 0.3400

---

## Dependency Advisory 005: GHSA-1005-2005-3005 | severity: CRITICAL

package: serde@1.5:5.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_05 -> serde
fixed_version: 1.0.52 | EPSS: 0.3500

---

## Dependency Advisory 006: GHSA-1006-2006-3006 | severity: CRITICAL

package: serde@1.6:8.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_06 -> serde
fixed_version: 1.0.52 | EPSS: 0.3600

---

## Dependency Advisory 007: GHSA-1007-2007-3007 | severity: CRITICAL

package: serde@1.7:1.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_07 -> serde
fixed_version: 1.0.52 | EPSS: 0.3700

---

## Dependency Advisory 008: GHSA-1008-2008-3008 | severity: CRITICAL

package: serde@1.8:4.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_08 -> serde
fixed_version: 1.0.52 | EPSS: 0.3800

---

## Dependency Advisory 009: GHSA-1009-2009-3009 | severity: CRITICAL

package: serde@1.0:7.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_09 -> serde
fixed_version: 1.0.52 | EPSS: 0.3900

---

## Dependency Advisory 010: GHSA-1010-2010-3010 | severity: CRITICAL

package: serde@1.1:0.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_10 -> serde
fixed_version: 1.0.52 | EPSS: 0.4000

---

## Dependency Advisory 011: GHSA-1011-2011-3011 | severity: CRITICAL

package: serde@1.2:3.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_11 -> serde
fixed_version: 1.0.52 | EPSS: 0.4100

---

## Dependency Advisory 012: GHSA-1012-2012-3012 | severity: CRITICAL

package: serde@1.3:6.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_12 -> serde
fixed_version: 1.0.52 | EPSS: 0.4200

---

## Dependency Advisory 013: GHSA-1013-2013-3013 | severity: CRITICAL

package: serde@1.4:9.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_13 -> serde
fixed_version: 1.0.52 | EPSS: 0.4300

---

## Dependency Advisory 014: GHSA-1014-2014-3014 | severity: CRITICAL

package: serde@1.5:2.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_14 -> serde
fixed_version: 1.0.52 | EPSS: 0.4400

---

## Dependency Advisory 015: GHSA-1015-2015-3015 | severity: CRITICAL

package: serde@1.6:5.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_15 -> serde
fixed_version: 1.0.52 | EPSS: 0.4500

---

## Dependency Advisory 016: GHSA-1016-2016-3016 | severity: CRITICAL

package: serde@1.7:8.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_16 -> serde
fixed_version: 1.0.52 | EPSS: 0.4600

---

## Dependency Advisory 017: GHSA-1017-2017-3017 | severity: CRITICAL

package: serde@1.8:1.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_17 -> serde
fixed_version: 1.0.52 | EPSS: 0.4700

---

## Dependency Advisory 018: GHSA-1018-2018-3018 | severity: CRITICAL

package: serde@1.9:4.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_18 -> serde
fixed_version: 1.0.52 | EPSS: 0.4800

---

## Dependency Advisory 019: GHSA-1019-2019-3019 | severity: CRITICAL

package: serde@1.0:7.0 | ecosystem: crates.io | vulnerable_versions: < 1.0.52
introduced via: package_a -> transitive_dep_19 -> serde
fixed_version: 1.0.52 | EPSS: 0.4900
