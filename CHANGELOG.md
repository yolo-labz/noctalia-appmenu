# Changelog

All notable changes to noctalia-appmenu are documented here. Format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) and adheres to
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### CI

- Land pages mdbook output inside the workspace (#136) ([77b3320](https://github.com/yolo-labz/noctalia-appmenu/commit/77b3320ab60276392625186b5238b59f3251d888))


### Chore

- **ci:** Ignore RUSTSEC-2025-0141 in osv-scanner.toml (#137) ([b66f7a1](https://github.com/yolo-labz/noctalia-appmenu/commit/b66f7a1978fd72ad4db38fd612c0769c535cf4e0))


### Documentation

- Capture v1.0.0..v1.0.6 release marathon postmortem (#135) ([c7b0de1](https://github.com/yolo-labz/noctalia-appmenu/commit/c7b0de17cd8dab3dc1e09e40806a0cea02ed3b22))

## [1.0.25] — 2026-05-23

### Chore

- **meta:** Ignore .claude session lock + local settings (#133) ([caa30fe](https://github.com/yolo-labz/noctalia-appmenu/commit/caa30fe1dca8e569f6929713c61a8eee5b0a8642))


### Features

- **bridge:** Frame-scoped menu resolution by focused-window title (#132) ([c0d72fd](https://github.com/yolo-labz/noctalia-appmenu/commit/c0d72fd199aaf67075c8cd184c45c11f351c5b6c))

## [1.0.24] — 2026-05-21

### Bug fixes

- **bridge:** Bump tokio 1.52.2 -> 1.52.3 (#69) ([e39d998](https://github.com/yolo-labz/noctalia-appmenu/commit/e39d9985171d7b65e38adb8bae79fbf224a9998d))

- **bridge:** Bump tempfile 3.20.0 -> 3.27.0 (#71) ([355b5fd](https://github.com/yolo-labz/noctalia-appmenu/commit/355b5fd42c057f1c5eece5511822cae5c57a21f5))

- **bridge:** Bump iai-callgrind 0.14.2 -> 0.16.1 (#70) ([caf58b0](https://github.com/yolo-labz/noctalia-appmenu/commit/caf58b0d7ea3ff58954ea5b53a913923e90f3e13))


### CI

- **deps:** Bump github/codeql-action from 4.35.4 to 4.35.5 (#104) ([35a84dd](https://github.com/yolo-labz/noctalia-appmenu/commit/35a84ddfed54346da5454d6f07f1854fd4dbaa37))

- **deps:** Bump codecov/codecov-action from 5.5.1 to 6.0.1 (#105) ([861c4dd](https://github.com/yolo-labz/noctalia-appmenu/commit/861c4dd818f1722f29660a42ba62ea194a5de3f1))

- **deps:** Bump step-security/harden-runner from 2.19.1 to 2.19.3 (#102) ([91bc9f7](https://github.com/yolo-labz/noctalia-appmenu/commit/91bc9f7bb9aad12767fc46ec95e54f7479d78633))

- **config:** Add deps scope to commitlint enum (#125) ([afde243](https://github.com/yolo-labz/noctalia-appmenu/commit/afde243681ed231c94603fc3286b424490002bbb))

- **config:** Ignore zbus + zvariant major bumps in dependabot (#126) ([503928a](https://github.com/yolo-labz/noctalia-appmenu/commit/503928a45088d0c8fe7673227dcd153f6209d75f))

- **deps:** Bump actions/upload-pages-artifact from 3.0.1 to 5.0.0 (#127) ([62e5da1](https://github.com/yolo-labz/noctalia-appmenu/commit/62e5da1270f2574a64b37ef15b28c843efcf665f))

- **deps:** Bump SonarSource/sonarqube-scan-action from 8.0.0 to 8.1.0 (#128) ([c397736](https://github.com/yolo-labz/noctalia-appmenu/commit/c397736462e32296e03f3f0577586df6a3fc8e93))

- **deps:** Bump cargo-deny-action SHA pin (#66) ([89119c5](https://github.com/yolo-labz/noctalia-appmenu/commit/89119c51f4069887dbb6772eb45e68c9a19ef73c))

- **deps:** Bump crate-ci/typos SHA pin (#103) ([d014c94](https://github.com/yolo-labz/noctalia-appmenu/commit/d014c94e7b7e0f281fc44bfd5d657e12ca3af40f))

- Fix read-only-HOME crash in codecov upload step (#130) ([c41bdd6](https://github.com/yolo-labz/noctalia-appmenu/commit/c41bdd6b3b77c6326459ef639ab2f1f4098402b9))


### Documentation

- **speckit:** Defer FR-003 accelerator dispatch (ADR-0028) (#121) ([fcbbaa3](https://github.com/yolo-labz/noctalia-appmenu/commit/fcbbaa3ac0989ea90378f45f98474ba1a5a00b0e))

- **speckit:** Spec 013 close-out plan (R1-R3 residue) (#122) ([7f34bad](https://github.com/yolo-labz/noctalia-appmenu/commit/7f34badb0b315e5bd649e50d301f007d8c655a34))

- **speckit:** Spec 015 doc-nit clean-up (C1 + T1) (#124) ([1642dfe](https://github.com/yolo-labz/noctalia-appmenu/commit/1642dfef11dd2760f32de5bd7a3adf4269f3d937))

- **speckit:** Close-out sweep for specs 001-008 (#123) ([7f4d863](https://github.com/yolo-labz/noctalia-appmenu/commit/7f4d863ab3712be371e4e72ed268fc44e3653e1d))

- **speckit:** Redesign spec for popup dismiss (spec 014) (#99) ([feb7262](https://github.com/yolo-labz/noctalia-appmenu/commit/feb72621c80ba4b4aef2d5009c147a98d2176e2e))


### Features

- **meta:** Drift trigger I + HM .backup pre-clear (#116) ([ce562f2](https://github.com/yolo-labz/noctalia-appmenu/commit/ce562f284730667687a1fb346e5ead317d1708ac))

- **meta:** Visual-audit + verify-tokens gate (FR-004/FR-010) (#118) ([4cf572c](https://github.com/yolo-labz/noctalia-appmenu/commit/4cf572ca154150888f31853b38d605aa4f480ab5))

- **meta:** Four release gates (FR-007) (#119) ([8e1917e](https://github.com/yolo-labz/noctalia-appmenu/commit/8e1917ed2fd322478888a6205f1f076ffc42eff4))

- **meta:** Verify-release driver + release.sh wire-up (FR-007) (#120) ([4af461f](https://github.com/yolo-labz/noctalia-appmenu/commit/4af461fdb9b3c0025de03c946402d5bd2d010264))

- **bridge:** Self-learning no-menubar skip replaces hardcoded list (#129) ([424570c](https://github.com/yolo-labz/noctalia-appmenu/commit/424570c8d580d8210f7b3d750e21ab771717d57b))

## [1.0.23] — 2026-05-20

### Bug fixes

- **plugin:** Self-heal hardening + cascade coverage (v1.0.23) (#117) ([6b52b5d](https://github.com/yolo-labz/noctalia-appmenu/commit/6b52b5dde2ecf171e414d2d79a631847fdae9f9a))


### Documentation

- **speckit:** Spec 015 — ship-ready completion scaffolding (#114) ([de370a1](https://github.com/yolo-labz/noctalia-appmenu/commit/de370a10349bbcc57e5dec07383d9353040e757f))

## [1.0.22] — 2026-05-20

### Bug fixes

- **bridge:** Focus-settle 30ms → 150ms (v1.0.22) (#115) ([13b92cf](https://github.com/yolo-labz/noctalia-appmenu/commit/13b92cf9d70514de7b23545304b4b16d85a50007))

## [1.0.21] — 2026-05-19

### Bug fixes

- **plugin:** Self-heal empty top-level on click (v1.0.21) (#113) ([7f2013b](https://github.com/yolo-labz/noctalia-appmenu/commit/7f2013bdc219353ed474bef4a7d4db14b469884b))

## [1.0.20] — 2026-05-19

### Bug fixes

- **bridge:** Pre-focus niri window before atspi DoAction (v1.0.20) (#112) ([8fcdd1f](https://github.com/yolo-labz/noctalia-appmenu/commit/8fcdd1f0194a3d7106279f233434cac25db81de5))


### Chore

- **config:** Point $schema at canonical schemastore URL (#111) ([1dec11f](https://github.com/yolo-labz/noctalia-appmenu/commit/1dec11f40c5e7bbffc7947f49472d0ea68a55d53))

## [1.0.19] — 2026-05-19

### Features

- **plugin:** MacOS-style bar attach — asymmetric corners (v1.0.19) (#110) ([797fc43](https://github.com/yolo-labz/noctalia-appmenu/commit/797fc433ef3c0506ffa7fb428f94a4031c7f29dc))

## [1.0.18] — 2026-05-19

### Features

- **release:** Canonical deploy skill + tag-subject pre-push guard (#106) ([18983f7](https://github.com/yolo-labz/noctalia-appmenu/commit/18983f7457effd8c99c0f9f0d424ccdc4fbc62d1))

- **release:** Cache-nuke stage — kill stale QML bytecode on deploy (#107) ([6611bb0](https://github.com/yolo-labz/noctalia-appmenu/commit/6611bb0aef7ee12d0a873a2e709c9831c5373784))

- **plugin:** Visual iter 2 — card-quality popup (v1.0.18) (#108) ([762cb1c](https://github.com/yolo-labz/noctalia-appmenu/commit/762cb1c9082f33df346cae00dd11eb9a0b64b35b))

## [1.0.17] — 2026-05-18

### Features

- **plugin:** Visual polish per visual-spec (v1.0.17) (#101) ([a411c71](https://github.com/yolo-labz/noctalia-appmenu/commit/a411c716159a294e10c3dff5b2a23e29bae5c545))

## [1.0.16] — 2026-05-18

### Features

- **plugin:** Option G full-screen transparent popup (v1.0.16) (#100) ([2bfc4df](https://github.com/yolo-labz/noctalia-appmenu/commit/2bfc4dfd0fa86088acf42771b00a9e9c42585fdb))

## [1.0.15] — 2026-05-18

### Bug fixes

- **plugin:** Shield to Overlay layer (v1.0.15) (#97) ([fe4f86a](https://github.com/yolo-labz/noctalia-appmenu/commit/fe4f86aec5a21eeeec426d71dd9ee72b12fc77ff))

## [1.0.14] — 2026-05-17

### Bug fixes

- **plugin:** Shield as belt-and-braces dismiss (v1.0.14) (#96) ([ed207c4](https://github.com/yolo-labz/noctalia-appmenu/commit/ed207c4cd4e2c322b4ec00f516e33638d784fe0c))

## [1.0.13] — 2026-05-17

### Bug fixes

- **plugin:** Explicit grabFocus on PopupWindow (v1.0.13) (#95) ([5fa350e](https://github.com/yolo-labz/noctalia-appmenu/commit/5fa350eee93e781f903fff9771ab8d9031d3c0f7))


### Documentation

- **meta:** Enforce drift triggers + decision tree (spec 013) (#94) ([3912ca7](https://github.com/yolo-labz/noctalia-appmenu/commit/3912ca70f52f5d02ee7010e5bd8c6843b7891321))

## [1.0.12] — 2026-05-17

### Bug fixes

- **plugin:** Xdg_popup grab + visual polish (v1.0.12) (#93) ([db8e3ef](https://github.com/yolo-labz/noctalia-appmenu/commit/db8e3ef1af48f4855eced7e945d7ead608ce7382))

## [1.0.11] — 2026-05-16

### Bug fixes

- **plugin:** Shield input via mask Region (v1.0.11) (#92) ([77ceb0c](https://github.com/yolo-labz/noctalia-appmenu/commit/77ceb0c85e002b5a2d43cd1bf3fb667a7bf0e8ca))

## [1.0.10] — 2026-05-16

### Bug fixes

- **plugin:** Popup→Overlay + permanent shield (v1.0.10) (#91) ([fdc6d2e](https://github.com/yolo-labz/noctalia-appmenu/commit/fdc6d2ec0fa6436f60967f6d9ae536ebc50218c1))

## [1.0.9] — 2026-05-16

### Bug fixes

- **plugin:** Outside-click dismisses appmenu popup (v1.0.9) (#90) ([b10a312](https://github.com/yolo-labz/noctalia-appmenu/commit/b10a312ef770d7829806a037235a10d1c4013e59))

## [1.0.8] — 2026-05-16

### Performance

- **bridge:** Parallel walk fixes Firefox blank submenus (v1.0.8) (#89) ([b68f889](https://github.com/yolo-labz/noctalia-appmenu/commit/b68f889317ddb9ece6847e77f889515814d4a745))

## [1.0.7] — 2026-05-16

### Bug fixes

- **bridge:** Restore Firefox + Chromium menus (v1.0.7) (#88) ([0be6327](https://github.com/yolo-labz/noctalia-appmenu/commit/0be6327e9b6c464292427422511eb062942ddcb6))

## [1.0.6] — 2026-05-16

### Performance

- **v1.0.6:** Skip-list + 30s cache + depth 3 (instant for known apps) (#87) ([8802541](https://github.com/yolo-labz/noctalia-appmenu/commit/8802541f5d8ffec63ed2563388c7912eeaf99395))

## [1.0.5] — 2026-05-16

### Bug fixes

- **v1.0.5:** Drop recursive Component (plugin actually loads now) (#86) ([36456d4](https://github.com/yolo-labz/noctalia-appmenu/commit/36456d49bec8dc4ff580dda4d8f61b60065e8a4f))

## [1.0.4] — 2026-05-16

### Bug fixes

- **v1.0.4:** Keep popup wl_surface mapped (codex review) (#85) ([81db398](https://github.com/yolo-labz/noctalia-appmenu/commit/81db39816b16401e29708df1fc10631d6c2f1024))

## [1.0.3] — 2026-05-16

### Bug fixes

- **v1.0.3:** Popup surface constraint - kill full-screen wl_surface (#84) ([d5dc002](https://github.com/yolo-labz/noctalia-appmenu/commit/d5dc002f251bc7693bb71682836637485eb093ef))

## [1.0.2] — 2026-05-15

### Bug fixes

- **v1.0.2:** Drop synthetic menu + bump menuBox contrast (#83) ([ef10f03](https://github.com/yolo-labz/noctalia-appmenu/commit/ef10f0303e792a94386baf1280a10525488b148f))

## [1.0.1] — 2026-05-15

### Bug fixes

- **v1.0.1:** Popup hotfix - width clamp + async cascade + dedup (#82) ([7e56d68](https://github.com/yolo-labz/noctalia-appmenu/commit/7e56d683be568fc6bf4fd21f244c5f8149cfa660))

## [1.0.0] — 2026-05-13

### Bug fixes

- **nix:** Drop osConfig from HM module — eval recursion (ADR-0027) (#80) ([2d30af6](https://github.com/yolo-labz/noctalia-appmenu/commit/2d30af60bc89f6b6f52b86e76d913e1244ab8152))


### CI

- **deps:** Bump actions/deploy-pages from 4.0.5 to 5.0.0 (#64) ([a953590](https://github.com/yolo-labz/noctalia-appmenu/commit/a9535901830114eb922112a49294a979912fccc7))

## [1.0.0-rc.2] — 2026-05-12

### Bug fixes

- **ci:** Emit CycloneDX 1.6 (syft constraint, ADR-0026) (#79) ([b52a807](https://github.com/yolo-labz/noctalia-appmenu/commit/b52a807970f66d14b4a78318ba9d03aff98ccfb6))

## [1.0.0-rc.1] — 2026-05-12

### CI

- **deps:** Bump step-security/harden-runner from 2.17.0 to 2.19.1 (#4) ([6451ad7](https://github.com/yolo-labz/noctalia-appmenu/commit/6451ad74c5154ee3bd479fd40c30f96031896239))

- **deps:** Bump SonarSource/sonarqube-scan-action from 7.1.0 to 8.0.0 (#6) ([bea3c76](https://github.com/yolo-labz/noctalia-appmenu/commit/bea3c76ba5e915e0bddd9c94b6e39777418dd63e))

- **deps:** Bump actions/checkout from 4.2.2 to 6.0.2 (#8) ([99f7428](https://github.com/yolo-labz/noctalia-appmenu/commit/99f7428df733d769a4699749bab3313b4d6c0d58))

- **deps:** Bump google/osv-scanner-action from 2.3.5 to 2.3.8 (#65) ([2ec63b1](https://github.com/yolo-labz/noctalia-appmenu/commit/2ec63b16a17b841d5dd3d5911ea53517427d7784))

- **deps:** Bump github/codeql-action from 4.35.3 to 4.35.4 (#72) ([1e72370](https://github.com/yolo-labz/noctalia-appmenu/commit/1e7237026c27e22e4f9e2fa2e0ad9f52d7aa4d46))

- V1.0.0 release engineering polish (Lane D) (#76) ([e9b0402](https://github.com/yolo-labz/noctalia-appmenu/commit/e9b04029875903814fe11fd33cc88088ec645132))


### Documentation

- **speckit:** Spec 004 — v1.0.0 project completion roadmap (#73) ([b720a53](https://github.com/yolo-labz/noctalia-appmenu/commit/b720a53c0e8892d23fe2ef1a2c9848654c6a6952))


### Features

- **plugin:** Nested submenus + toggle_state + icon_name + screen guard (#74) ([a1093fe](https://github.com/yolo-labz/noctalia-appmenu/commit/a1093fe3a4c55545e2617fe935b1c2c18133c1de))

- **nix:** AT-SPI prerequisites + flake hygiene (Lane C of v1.0.0) (#75) ([c743087](https://github.com/yolo-labz/noctalia-appmenu/commit/c74308706c45d9612cd4bb507f7032a837805cd5))

- **bridge:** Focus tracker + AT-SPI walker (Lane A) (#77) ([dcaa27e](https://github.com/yolo-labz/noctalia-appmenu/commit/dcaa27e286f7bf3ddea422a7825baf0d6d9808f5))

## [0.3.0] — 2026-05-10

### Bug fixes

- **plugin:** Stable-slot animated width to eliminate full-screen flicker (#51) ([7d133be](https://github.com/yolo-labz/noctalia-appmenu/commit/7d133beb5a1d16d60c00c83ece22fafe327dcae9))

- **plugin:** Top-level PanelWindow dropdown to keep bar clickable (#56) ([dcde40a](https://github.com/yolo-labz/noctalia-appmenu/commit/dcde40aa7b5434865ef35847563257b5482bf232))

- **plugin:** Isolation envelope around applySnapshot defers via Qt.callLater (#57) ([35f0762](https://github.com/yolo-labz/noctalia-appmenu/commit/35f076264e604718c76517764d2a00c8fd614364))


### CI

- Force --noprofile --norc bash for self-hosted runner steps (#53) ([1df752a](https://github.com/yolo-labz/noctalia-appmenu/commit/1df752a7b9b6f6634482688a7c67fb952f819eb8))


### Documentation

- **speckit:** Spec 003 plugin fault-isolation envelope (#55) ([340cef5](https://github.com/yolo-labz/noctalia-appmenu/commit/340cef5b472a57e2f468a1c0c99e47646bfd94b9))


### Features

- **bridge:** Active.json schema v=1 + producer-side dedup (#59) ([6ad9c2d](https://github.com/yolo-labz/noctalia-appmenu/commit/6ad9c2dc467c596261fbe7190aa5020a0b9d7973))


### Refactor

- **bridge:** Adopt niri-ipc crate for socket + state types (#54) ([9d666b1](https://github.com/yolo-labz/noctalia-appmenu/commit/9d666b1850ae5496410e0fde3f27c6bd854af9e2))


### Tests

- **bridge:** Fixture-replay test harness for niri event-stream loop (#60) ([b6097cc](https://github.com/yolo-labz/noctalia-appmenu/commit/b6097cc453789712476ec879ea3ce2c98cca1499))


### Style

- **bridge:** Post-niri-ipc cleanup — fmt + deny.toml + bench rewrite (#62) ([35e3eb8](https://github.com/yolo-labz/noctalia-appmenu/commit/35e3eb8704ec338c2f4ce1dc26eee2af3fb66233))

## [0.3.0-alpha.17] — 2026-05-10

### Bug fixes

- **plugin:** Eliminate click dead-zone above/below bar buttons (#50) ([efcc865](https://github.com/yolo-labz/noctalia-appmenu/commit/efcc8652ef93c7f1d6f44e6dd15f00eb116e5402))

## [0.3.0-alpha.16] — 2026-05-10

### Bug fixes

- **plugin:** Use anchor.item instead of anchor.window for popup (#49) ([df18414](https://github.com/yolo-labz/noctalia-appmenu/commit/df1841407e61ee84c2a3615662aa3dbd8e35c167))

## [0.3.0-alpha.15] — 2026-05-10

### Features

- **nix:** NiriPackage option + PATH default for niri_binary (#48) ([dd58673](https://github.com/yolo-labz/noctalia-appmenu/commit/dd586737cb02c8135918e8549aeba7fdcfaf88ce))

## [0.3.0-alpha.14] — 2026-05-10

### Bug fixes

- **bridge:** Respawn niri event-stream on child crash (#46) ([cecb1a8](https://github.com/yolo-labz/noctalia-appmenu/commit/cecb1a8d3010a1637b9e059ea919441418acc1ea))


### Features

- Split-the-loss UX — hide widget when no menu, type IpcHandler arg (#47) ([252c997](https://github.com/yolo-labz/noctalia-appmenu/commit/252c997f2baaa7a33e13bd17864946acf33b7c5d))

## [0.3.0-alpha.11] — 2026-05-08

### Features

- IpcHandler push channel + honest-only synthetic menu (#44) ([2ed78c2](https://github.com/yolo-labz/noctalia-appmenu/commit/2ed78c269458f1ac2780f0d1dcf5a227300404aa))

## [0.3.0-alpha.10] — 2026-05-07

### Features

- **bridge:** MacOS-style synthetic menu — Application + Edit + Window (#43) ([50a5555](https://github.com/yolo-labz/noctalia-appmenu/commit/50a55550f60613fa12f49764b950c41664a78fd1))

## [0.3.0-alpha.9] — 2026-05-07

### Features

- **bridge:** Synthetic Window menu fallback for non-AT-SPI apps (#42) ([8a4fdc9](https://github.com/yolo-labz/noctalia-appmenu/commit/8a4fdc951d12204108755252d2428e54c0def334))

## [0.3.0-alpha.8] — 2026-05-06

### Bug fixes

- **bridge:** In-place active.json write to keep inotify watch alive (#41) ([d0cbac2](https://github.com/yolo-labz/noctalia-appmenu/commit/d0cbac22cd118303382a73d2c2e40954002bc668))

## [0.3.0-alpha.7] — 2026-05-06

### Bug fixes

- **bridge:** Cancellable retry + 3s budget for cold AT-SPI registry (#40) ([62f1ea6](https://github.com/yolo-labz/noctalia-appmenu/commit/62f1ea68a85b5eb3957db077c505b60e6c65629d))

## [0.3.0-alpha.6] — 2026-05-06

### Bug fixes

- **bridge:** Gate STATE_ACTIVE on app_id corroboration (#39) ([0816386](https://github.com/yolo-labz/noctalia-appmenu/commit/08163863b4e988f1e54b63f8c9c26eb13af457ba))

## [0.3.0-alpha.5] — 2026-05-06

### Features

- **bridge:** Universal active-app via AT-SPI STATE_ACTIVE walk (#38) ([531a59a](https://github.com/yolo-labz/noctalia-appmenu/commit/531a59a116f6d46a8fa1ef7d94c387b7ad1c738d))

## [0.3.0-alpha.4] — 2026-05-06

### Features

- **bridge:** XWayland PID fallback via app_id name match (#37) ([9c18b5c](https://github.com/yolo-labz/noctalia-appmenu/commit/9c18b5c3ca78ed32a9b137ee177af1bcfaeebf01))

## [0.3.0-alpha.3] — 2026-05-06

### Chore

- **bridge:** Bump Cargo.toml to 0.3.0-alpha.2 (#36) ([4856271](https://github.com/yolo-labz/noctalia-appmenu/commit/48562718ddb5ad2cfc6c2a092856086595450622))

## [0.3.0-alpha.2] — 2026-05-06

### Bug fixes

- **bridge:** Codex P0+P1 fixes for v0.3 substrate (#35) ([a26af1f](https://github.com/yolo-labz/noctalia-appmenu/commit/a26af1fda61c903ad8f98776bac607be184e659e))


### CI

- **machete:** Use nix devShell + drop unused futures dep (#33) ([f68a602](https://github.com/yolo-labz/noctalia-appmenu/commit/f68a602b01b05063e89f761b89759d41a9927b85))


### Chore

- **bridge:** Apply clippy --fix --pedantic auto-fixes (#34) ([04e1a84](https://github.com/yolo-labz/noctalia-appmenu/commit/04e1a84f5ea322e629bb8fc0a16a1b10f0a9c135))

## [0.3.0-alpha.1] — 2026-05-06

### Features

- **bridge:** V0.3 AT-SPI substrate replaces DBusMenu mirror (#32) ([ab80c3d](https://github.com/yolo-labz/noctalia-appmenu/commit/ab80c3d46e62aaf5d98e678c458cb4980cff9fb3))

## [0.2.0-alpha.1] — 2026-05-06

### Features

- **bridge:** Own com.canonical.AppMenu.Registrar (#29) ([fcb9a87](https://github.com/yolo-labz/noctalia-appmenu/commit/fcb9a8750916cda6bc2eca0d2c2f7400c4d43266))

- **bridge:** Fetch DBusMenu trees on focus (phase B+C) (#30) ([780333e](https://github.com/yolo-labz/noctalia-appmenu/commit/780333ebda393df8c1abbc6ff57180d6bfa2e9ff))

- **plugin:** V0.2 menu strip + click forward (#31) ([3192601](https://github.com/yolo-labz/noctalia-appmenu/commit/31926017729c8ebfd708bb10e11384052de96fbe))

## [0.1.9] — 2026-05-05

### Bug fixes

- **plugin:** FileView text() function (not property) (#28) ([8507425](https://github.com/yolo-labz/noctalia-appmenu/commit/8507425c8ca53779477e576174f62d33cd6afee2))

## [0.1.8] — 2026-05-05

### Bug fixes

- **plugin:** Pin slot width (defeat cache race) (#27) ([b86c6e3](https://github.com/yolo-labz/noctalia-appmenu/commit/b86c6e34b288bf01642e35a9cb026431893962e2))

## [0.1.7] — 2026-05-05

### Bug fixes

- **plugin:** Always-visible BarWidget (drop visibility gate) (#26) ([c46a94d](https://github.com/yolo-labz/noctalia-appmenu/commit/c46a94d5bc456b5b07cdaa451ac87dc51a1c7415))

## [0.1.6] — 2026-05-05

### Bug fixes

- **plugin:** BarWidget declares the bar-widget API contract (#25) ([f249a99](https://github.com/yolo-labz/noctalia-appmenu/commit/f249a995cb99c3e995e0dc17e6d3c77f02e4cfe1))

## [0.1.5] — 2026-05-05

### Bug fixes

- **plugin:** Manifest schema matches noctalia-shell loader (#24) ([89b9855](https://github.com/yolo-labz/noctalia-appmenu/commit/89b98554731f16f443655e9932ec699fec6c83fc))

## [0.1.4] — 2026-05-05

### Bug fixes

- **bridge:** Niri event-stream uses externally-tagged JSON enum (#23) ([9d93bcf](https://github.com/yolo-labz/noctalia-appmenu/commit/9d93bcf0f1186154cfb19669a0ce6f290ffd45c2))


### CI

- **deps:** Bump github/codeql-action from 3.28.18 to 4.35.3 (#9) ([ceec9a7](https://github.com/yolo-labz/noctalia-appmenu/commit/ceec9a77df6e3d1aac5114f36cc6fec311d6121a))

## [0.1.3] — 2026-05-05

### Bug fixes

- **bridge:** Bail on unexpected task exit (systemd restart) (#22) ([895aa0e](https://github.com/yolo-labz/noctalia-appmenu/commit/895aa0e35e74fb5321f1cb9c58337acae25a6739))

- **plugin:** BarWidget FileView rewrite (DBusObject not in Quickshell) (#21) ([4c562ca](https://github.com/yolo-labz/noctalia-appmenu/commit/4c562ca6e478087ee44eb1c6cf2be2e60948bb51))

## [0.1.2] — 2026-05-05

### Bug fixes

- **nix:** ToTOML→pkgs.formats.toml (HM module integration) (#20) ([8af33cd](https://github.com/yolo-labz/noctalia-appmenu/commit/8af33cd5effd626019ef24168f96503513ea0cf8))


### CI

- **deps:** Bump actions/upload-artifact from 4.6.2 to 7.0.1 (#10) ([b0f7374](https://github.com/yolo-labz/noctalia-appmenu/commit/b0f73746c9af6eedd8db1178bb578a5053077212))

## [0.1.1] — 2026-05-05

### Bug fixes

- **ci:** Syft cdx 1.7→1.6 + typos sha + drop quickshell (#19) ([b880388](https://github.com/yolo-labz/noctalia-appmenu/commit/b8803882fef02b47bbb7c2e7667016e98898907d))

## [0.1.0] — 2026-05-04

### CI

- Runner-agnostic labels (drop vm103) — unblocks queue (#13) ([d7894df](https://github.com/yolo-labz/noctalia-appmenu/commit/d7894df64be6c2a587c157aaf5f9cca425781a36))


### Chore

- Initial scaffold — bridge + plugin + CI/CD + speckit ([ca7b83d](https://github.com/yolo-labz/noctalia-appmenu/commit/ca7b83d8518e013de015cf0ad48a4c549378bed4))


### Features

- **v0.1.0:** Consolidated fix-up — all 13 quality gates green locally (#18) ([005ddb1](https://github.com/yolo-labz/noctalia-appmenu/commit/005ddb1c5710c26e54ac6388fe68f2a529568872))

