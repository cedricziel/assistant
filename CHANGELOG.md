# Changelog

## [0.1.152](https://github.com/cedricziel/assistant/compare/v0.1.151...v0.1.152) (2026-05-16)


### Features

* **runtime:** auto-title conversations via turn.result consumer ([#722](https://github.com/cedricziel/assistant/issues/722)) ([a6528ef](https://github.com/cedricziel/assistant/commit/a6528efc9a0c63183b133691753d958b558144ab))


### Bug Fixes

* **app:** web favicon + PWA maskable icons match brand ([#721](https://github.com/cedricziel/assistant/issues/721)) ([b186134](https://github.com/cedricziel/assistant/commit/b18613440a10b83497be8972eb09977eeaef7729))
* **chat:** drop empty bubbles for tool-only assistant history rows ([#717](https://github.com/cedricziel/assistant/issues/717)) ([73c75d6](https://github.com/cedricziel/assistant/commit/73c75d68784a3e7ba86cc717897b69237b66c866))

## [0.1.151](https://github.com/cedricziel/assistant/compare/v0.1.150...v0.1.151) (2026-05-10)


### Bug Fixes

* **app:** login form Tab traversal + password manager autofill ([#695](https://github.com/cedricziel/assistant/issues/695)) ([589aa99](https://github.com/cedricziel/assistant/commit/589aa99661fc455984b4292a0d1209b439563a10))
* **packaging:** detect legacy units via wants/ symlinks during migration ([#700](https://github.com/cedricziel/assistant/issues/700)) ([719f29f](https://github.com/cedricziel/assistant/commit/719f29f077479fb4393b4f99cbf45d9de7878963))

## [0.1.150](https://github.com/cedricziel/assistant/compare/v0.1.149...v0.1.150) (2026-05-10)


### Features

* **packaging:** single orchestrator unit + ASSISTANT_INTERFACES env ([#697](https://github.com/cedricziel/assistant/issues/697)) ([164896c](https://github.com/cedricziel/assistant/commit/164896c0782749fe8a09b1347ac5df296d2db908))
* **web-ui:** --public-url for OAuth issuer + agent card ([#696](https://github.com/cedricziel/assistant/issues/696)) ([626ea24](https://github.com/cedricziel/assistant/commit/626ea24b1d6cfc4c46f864941c62ce48259898af))

## [0.1.149](https://github.com/cedricziel/assistant/compare/v0.1.148...v0.1.149) (2026-05-10)


### Bug Fixes

* **app:** logout button missing on macOS / iOS Cupertino paths ([#693](https://github.com/cedricziel/assistant/issues/693)) ([d2681db](https://github.com/cedricziel/assistant/commit/d2681db3171a07f64b3b49c3af6ea8a8fcae8fba))

## [0.1.148](https://github.com/cedricziel/assistant/compare/v0.1.147...v0.1.148) (2026-05-10)


### Features

* **app:** expose logout on native (rename performWebLogout → performLogout) ([#689](https://github.com/cedricziel/assistant/issues/689)) ([e1456d9](https://github.com/cedricziel/assistant/commit/e1456d930eb29d50094849aafa115758361aec23))
* **app:** use EventSource for SSE on web (conversation list) ([#692](https://github.com/cedricziel/assistant/issues/692)) ([3d74042](https://github.com/cedricziel/assistant/commit/3d740420e928bfc68ba307788c7ddbc1387551c6))


### Bug Fixes

* **app:** close affordance + hide change-org for single-org users ([#690](https://github.com/cedricziel/assistant/issues/690)) ([ed35cae](https://github.com/cedricziel/assistant/commit/ed35cae430b29ff67d47e4bf7baf6ac57431438d))

## [0.1.147](https://github.com/cedricziel/assistant/compare/v0.1.146...v0.1.147) (2026-05-09)


### Bug Fixes

* **app:** web session resilience — corruption detection, banner, persistence, SW versioning ([#687](https://github.com/cedricziel/assistant/issues/687)) ([dc25b9a](https://github.com/cedricziel/assistant/commit/dc25b9ac08aec0abcd50d3f43fcc8d03f6e5b8f4))

## [0.1.146](https://github.com/cedricziel/assistant/compare/v0.1.145...v0.1.146) (2026-05-09)


### Features

* **web-ui:** document 401 on multi-org endpoints + ErrorBody envelope ([#686](https://github.com/cedricziel/assistant/issues/686)) ([69fa27f](https://github.com/cedricziel/assistant/commit/69fa27fe8e35f3727e2a641845043fabce64f843))


### Bug Fixes

* **app:** handle 401s and space-selector race conditions ([#685](https://github.com/cedricziel/assistant/issues/685)) ([14a6d34](https://github.com/cedricziel/assistant/commit/14a6d34360f4c23ef34d4b65b59a380a5e7dfc59))
* **integration-tests:** bump LLM timeout and cap iterations for CPU CI ([#683](https://github.com/cedricziel/assistant/issues/683)) ([72d0e0f](https://github.com/cedricziel/assistant/commit/72d0e0ff53ab817cd71a6401f24617aa2cf54526))

## [0.1.145](https://github.com/cedricziel/assistant/compare/v0.1.144...v0.1.145) (2026-04-28)


### Bug Fixes

* **web-ui:** JWT carries real org_id from user record ([#676](https://github.com/cedricziel/assistant/issues/676)) ([a60c431](https://github.com/cedricziel/assistant/commit/a60c4312f467cad9e0d6d04eaeba58786fa194cd))

## [0.1.144](https://github.com/cedricziel/assistant/compare/v0.1.143...v0.1.144) (2026-04-28)


### Features

* **runtime:** bound LLM retry/redelivery to prevent credit drain ([#672](https://github.com/cedricziel/assistant/issues/672)) ([fc77ef5](https://github.com/cedricziel/assistant/commit/fc77ef5d88d698cb816953af166e290afac09ab8))

## [0.1.143](https://github.com/cedricziel/assistant/compare/v0.1.142...v0.1.143) (2026-04-26)


### Features

* **multi-org:** runtime cutover, migrate finalize, doctor drift ([#668](https://github.com/cedricziel/assistant/issues/668)) ([a5bba84](https://github.com/cedricziel/assistant/commit/a5bba84f1c2a35ace9598f3cddc8f1c219f1a8a9))

## [0.1.142](https://github.com/cedricziel/assistant/compare/v0.1.141...v0.1.142) (2026-04-26)


### Features

* **packaging:** ship assistant-matrix systemd user unit ([bb57a92](https://github.com/cedricziel/assistant/commit/bb57a9239cc9b0e14adada77ad826040a04b3c25))

## [0.1.141](https://github.com/cedricziel/assistant/compare/v0.1.140...v0.1.141) (2026-04-26)


### Features

* **app:** self-service Account screen for name/email/password ([#662](https://github.com/cedricziel/assistant/issues/662)) ([68160ef](https://github.com/cedricziel/assistant/commit/68160eff277a87f75aed4a773111644f8a44c9bd))
* **auth:** self-service-account proposal + bulk refresh-token revocation ([#658](https://github.com/cedricziel/assistant/issues/658)) ([7cb55fe](https://github.com/cedricziel/assistant/commit/7cb55fedb0557c3024bf2803308b890339ef3e9f))
* **cli:** `assistant account` subcommand ([#663](https://github.com/cedricziel/assistant/issues/663)) ([35e4df1](https://github.com/cedricziel/assistant/commit/35e4df1f291914ab86857508c35876dddee8cd55))
* **web-ui:** self-service /api/users/me account endpoints ([#660](https://github.com/cedricziel/assistant/issues/660)) ([f007aa2](https://github.com/cedricziel/assistant/commit/f007aa206164c98bad463448cedeb6e77489ed3d))

## [0.1.140](https://github.com/cedricziel/assistant/compare/v0.1.139...v0.1.140) (2026-04-25)


### Bug Fixes

* **oauth:** OpenAPI annotation fixes and client registration hardening ([#655](https://github.com/cedricziel/assistant/issues/655)) ([d2fafd5](https://github.com/cedricziel/assistant/commit/d2fafd595fbe3459d5e9143e42261cc50b086621))

## [0.1.139](https://github.com/cedricziel/assistant/compare/v0.1.138...v0.1.139) (2026-04-25)


### Features

* **app:** switch OAuthService to generated OauthApi client ([#654](https://github.com/cedricziel/assistant/issues/654)) ([45a8949](https://github.com/cedricziel/assistant/commit/45a89493725b7e500395c70fd1517afba13073ac))


### Bug Fixes

* **auth:** harden OAuth2 — mandatory PKCE, redirect validation, open redirect fix ([#653](https://github.com/cedricziel/assistant/issues/653)) ([777d143](https://github.com/cedricziel/assistant/commit/777d14311362e86ea6669bfb32ceb0975ab1d2ed))
* **web-ui:** use OrgPoolFactory for org.db resolution ([#651](https://github.com/cedricziel/assistant/issues/651)) ([613a429](https://github.com/cedricziel/assistant/commit/613a42952460533b6424abc87faf6fc6af4398f2))

## [0.1.138](https://github.com/cedricziel/assistant/compare/v0.1.137...v0.1.138) (2026-04-25)


### Bug Fixes

* **app:** handle Dio redirect exceptions in OAuth2 login ([#649](https://github.com/cedricziel/assistant/issues/649)) ([cc8bb9f](https://github.com/cedricziel/assistant/commit/cc8bb9fc76f2050667125ef1014472365b778188))

## [0.1.137](https://github.com/cedricziel/assistant/compare/v0.1.136...v0.1.137) (2026-04-25)


### Bug Fixes

* **storage:** create org.db at install root during migration ([#646](https://github.com/cedricziel/assistant/issues/646)) ([d2752d9](https://github.com/cedricziel/assistant/commit/d2752d9fd48b82adc68f26f3f1b617416a2e55cd))
* **storage:** OrgPoolFactory.org_db_path() returns root-level path ([#648](https://github.com/cedricziel/assistant/issues/648)) ([8852670](https://github.com/cedricziel/assistant/commit/8852670ae3172cfa49f09be4968158f68bab0941))

## [0.1.136](https://github.com/cedricziel/assistant/compare/v0.1.135...v0.1.136) (2026-04-25)


### Features

* **app:** Flutter multi-user UI — space switcher, API keys, admin screens ([#644](https://github.com/cedricziel/assistant/issues/644)) ([5fa17f6](https://github.com/cedricziel/assistant/commit/5fa17f60527f37d6bf40cd6b482b92f1d2018bd9))
* **app:** OAuth2 login flow with PKCE and dual-mode auth ([#642](https://github.com/cedricziel/assistant/issues/642)) ([0e667a7](https://github.com/cedricziel/assistant/commit/0e667a747632b3bc6513819a94442a5a95b37a8f))
* **app:** org/space selector after OAuth2 login ([#643](https://github.com/cedricziel/assistant/issues/643)) ([d25761c](https://github.com/cedricziel/assistant/commit/d25761cd14305ebb25cafabbdc104fb56e74159c))
* **app:** theme-aware Mermaid diagrams from ColorScheme ([#637](https://github.com/cedricziel/assistant/issues/637)) ([8184d6b](https://github.com/cedricziel/assistant/commit/8184d6babfff231f24ecb3bae445d6a0b2071cb5))
* **auth:** OAuth2 server — PKCE, auth code grant, device code, client registration ([#612](https://github.com/cedricziel/assistant/issues/612)) ([902f791](https://github.com/cedricziel/assistant/commit/902f7914f684084e48c4e172827be6a9351949c4))
* **auth:** OIDC federation, API keys, middleware, and AuthProvider implementations ([#615](https://github.com/cedricziel/assistant/issues/615)) ([581f260](https://github.com/cedricziel/assistant/commit/581f260ca3a6b9d577d5f544c431a8df8e1d7e6c))
* **cli:** add `assistant doctor` subcommand ([#632](https://github.com/cedricziel/assistant/issues/632)) ([adc3b5f](https://github.com/cedricziel/assistant/commit/adc3b5f3665c41eecaa2b04ec83f95805ffd7c17))
* **cli:** OAuth2 device code login, api-keys, and credential storage ([#639](https://github.com/cedricziel/assistant/issues/639)) ([9ab0f3c](https://github.com/cedricziel/assistant/commit/9ab0f3c31380ef02eec975e26e0ef97ceb9d93fa))
* core identity types, auth abstractions, and assistant-auth crate ([#610](https://github.com/cedricziel/assistant/issues/610)) ([0916a4d](https://github.com/cedricziel/assistant/commit/0916a4de99974979660ea5cf65c15d0de03348af))
* **llm-provider:** add OpenRouter provider with shared Chat Completions base ([#633](https://github.com/cedricziel/assistant/issues/633)) ([a59bb43](https://github.com/cedricziel/assistant/commit/a59bb43bf112e216057dc0393a5f6b27dd410c3c))
* **openspec:** rich output rendering — SVG + interface-aware capabilities ([#613](https://github.com/cedricziel/assistant/issues/613)) ([617f86a](https://github.com/cedricziel/assistant/commit/617f86ad90ebabcd1b6321b47c9d18617daa6e90))
* **runtime:** runtime identity threading — ChannelRunner, scheduler, adapter registry ([#636](https://github.com/cedricziel/assistant/issues/636)) ([621ff11](https://github.com/cedricziel/assistant/commit/621ff1150c3605afa7b96a05d3fd66172aaa72cb))
* **runtime:** storage migration + runtime AuthContext threading ([#634](https://github.com/cedricziel/assistant/issues/634)) ([a2f95c5](https://github.com/cedricziel/assistant/commit/a2f95c568f1a8f066f32928b41527c652bf67a98))
* **storage:** conversation/persona user-scoping and message sender identity ([#617](https://github.com/cedricziel/assistant/issues/617)) ([a42374f](https://github.com/cedricziel/assistant/commit/a42374fc45902c5da88a49d407d5c8fff560931d))
* **storage:** org/space database layer with multi-tenant stores ([#616](https://github.com/cedricziel/assistant/issues/616)) ([c1bbb89](https://github.com/cedricziel/assistant/commit/c1bbb89f04ad1a594a8ae4ca1b687f08d32c1e59))
* **web-ui:** auth middleware rewrite — JWT, API key, permission guards, session cookies ([#621](https://github.com/cedricziel/assistant/issues/621)) ([bcf19bd](https://github.com/cedricziel/assistant/commit/bcf19bd0666d5534c9050aa1d122a7756b9847cf))
* **web-ui:** auto-migrate legacy layout on startup ([#645](https://github.com/cedricziel/assistant/issues/645)) ([e7249f1](https://github.com/cedricziel/assistant/commit/e7249f1cfe1adfcebe1393b641bcf6592e7f30c0))
* **web-ui:** catalog, interfaces, bindings, templates API with AuthContext ([#630](https://github.com/cedricziel/assistant/issues/630)) ([3c8cfe8](https://github.com/cedricziel/assistant/commit/3c8cfe83553e84711eb6d8ca115884f914703b62))
* **web-ui:** management API endpoints for orgs, users, spaces, members, API keys ([#629](https://github.com/cedricziel/assistant/issues/629)) ([d18e68d](https://github.com/cedricziel/assistant/commit/d18e68d40286a40633ae54f1df891eb33dd71051))
* **web-ui:** OAuth2 endpoints — authorize, token, device flow, registration, revocation ([#618](https://github.com/cedricziel/assistant/issues/618)) ([3e7d565](https://github.com/cedricziel/assistant/commit/3e7d5657d687d3886b1514be3c36ccb4b9887490))
* **web-ui:** OIDC IdP callback — complete external IdP login flow ([#626](https://github.com/cedricziel/assistant/issues/626)) ([3ea6165](https://github.com/cedricziel/assistant/commit/3ea616569bf982791a0e396bc00c5dccb6aabf58))


### Bug Fixes

* **runtime:** prevent context window overflow with provider-aware compaction ([#635](https://github.com/cedricziel/assistant/issues/635)) ([782c6dd](https://github.com/cedricziel/assistant/commit/782c6dd76677c9efd837b5dc7f39701e4c17a591))
* **web-ui:** restore /setup auto-connect and update screenshot baselines ([#631](https://github.com/cedricziel/assistant/issues/631)) ([0afac30](https://github.com/cedricziel/assistant/commit/0afac3075853e7a46703656d95aca64a0a079a8b))

## [0.1.135](https://github.com/cedricziel/assistant/compare/v0.1.134...v0.1.135) (2026-04-22)


### Features

* **web-ui:** route traces, logs, and analytics through pluggable backends ([#605](https://github.com/cedricziel/assistant/issues/605)) ([1583bc1](https://github.com/cedricziel/assistant/commit/1583bc171b1e5bd639faf2236b1486584ff05c18))


### Bug Fixes

* **app:** share extension credential sync — direct Keychain writes via IOSOptions ([#606](https://github.com/cedricziel/assistant/issues/606)) ([d48ffff](https://github.com/cedricziel/assistant/commit/d48ffffd55dc58ead7d6e1fbf0843ab6a27673ff))

## [0.1.134](https://github.com/cedricziel/assistant/compare/v0.1.133...v0.1.134) (2026-04-22)


### Features

* **app:** SSE client robustness — backoff, auto-reconnect, heartbeat timeout ([#604](https://github.com/cedricziel/assistant/issues/604)) ([c9fb3d5](https://github.com/cedricziel/assistant/commit/c9fb3d51a75641c31ccd5693bd797d29299e006c))
* consolidate five interface crates into assistant-interfaces ([#601](https://github.com/cedricziel/assistant/issues/601)) ([3225eb3](https://github.com/cedricziel/assistant/commit/3225eb3a9bf3be1de658f1cac04193eebaae245c))

## [0.1.133](https://github.com/cedricziel/assistant/compare/v0.1.132...v0.1.133) (2026-04-22)


### Features

* consolidate LLM providers and move types to core ([#599](https://github.com/cedricziel/assistant/issues/599)) ([1b3c23b](https://github.com/cedricziel/assistant/commit/1b3c23bf3f857ef02bcc9053961be64d3d081258))
* **runtime:** crash recovery for orphaned SSE runs and server-side sequence tracking ([#600](https://github.com/cedricziel/assistant/issues/600)) ([3814720](https://github.com/cedricziel/assistant/commit/38147208aa0132f0186e577c5deb1759aa958977))
* **runtime:** propagate tool_call_id for stable tool-call correlation ([#597](https://github.com/cedricziel/assistant/issues/597)) ([bc70c33](https://github.com/cedricziel/assistant/commit/bc70c3389fdf2cba4897d9c6f275ea9c231d7bf8))


### Bug Fixes

* **app:** render tool calls as timeline entries and fix streaming progress indicator ([#595](https://github.com/cedricziel/assistant/issues/595)) ([aa706ee](https://github.com/cedricziel/assistant/commit/aa706ee72e8fda71572d7d8589740d624e0cedb6))
* **web-ui:** align SSE with spec and harden for production ([#598](https://github.com/cedricziel/assistant/issues/598)) ([19c9898](https://github.com/cedricziel/assistant/commit/19c98987739d070abadc3ec0b97151e3ca3912df))

## [0.1.132](https://github.com/cedricziel/assistant/compare/v0.1.131...v0.1.132) (2026-04-21)


### Features

* **workflows:** workflow editor UX improvements and node disable ([#592](https://github.com/cedricziel/assistant/issues/592)) ([2de6289](https://github.com/cedricziel/assistant/commit/2de62893c989354dfaf38729be63f6d7638a97a0))


### Bug Fixes

* **app:** retain chat input focus after sending and improve cursor visibility ([#593](https://github.com/cedricziel/assistant/issues/593)) ([06b6572](https://github.com/cedricziel/assistant/commit/06b657213ef6b301d42ef004620e5c2a5f024b2d))
* **ci:** disable code signing for ShareExtension in macOS CI build ([#588](https://github.com/cedricziel/assistant/issues/588)) ([47be819](https://github.com/cedricziel/assistant/commit/47be819c72e2bc9a9078b2141f912df8e0b7dc17))
* **integration-tests:** speed up smoke tests on CPU-only CI runners ([#589](https://github.com/cedricziel/assistant/issues/589)) ([e037b71](https://github.com/cedricziel/assistant/commit/e037b71092fccbc9554e21f02d075078847f28fa))

## [0.1.131](https://github.com/cedricziel/assistant/compare/v0.1.130...v0.1.131) (2026-04-21)


### Features

* **app:** add Cmd+N / Ctrl+N shortcut to create new chat ([#587](https://github.com/cedricziel/assistant/issues/587)) ([90877f2](https://github.com/cedricziel/assistant/commit/90877f2ee6335d3404e3ff4dfed9f4f704823f8e))
* **app:** add collapsible sidebar on wide screens ([#585](https://github.com/cedricziel/assistant/issues/585)) ([53ccd78](https://github.com/cedricziel/assistant/commit/53ccd7860398a9eae49cd48739b815232abbcc6d))
* **runtime:** enable skill learning for subagent turns ([#582](https://github.com/cedricziel/assistant/issues/582)) ([53a2471](https://github.com/cedricziel/assistant/commit/53a2471f148eea9a6feb02c738fb61e8a65633aa))


### Bug Fixes

* **flutter:** surface clear error on 401 in conversation stream ([#584](https://github.com/cedricziel/assistant/issues/584)) ([446f002](https://github.com/cedricziel/assistant/commit/446f0022cc334c6c7a28d37be386e056249a7ae2))
* **runtime:** prevent orphaned tool results from breaking Moonshot API ([#586](https://github.com/cedricziel/assistant/issues/586)) ([3b5661a](https://github.com/cedricziel/assistant/commit/3b5661a6eeb179f55bf1bd05e5768ae13aabea0b))

## [0.1.130](https://github.com/cedricziel/assistant/compare/v0.1.129...v0.1.130) (2026-04-21)


### Bug Fixes

* **ci:** move macOS signing and notarization into fastlane ([#580](https://github.com/cedricziel/assistant/issues/580)) ([26187eb](https://github.com/cedricziel/assistant/commit/26187ebd5d63edd9c1dc51f810db8dc6ec09bf40))

## [0.1.129](https://github.com/cedricziel/assistant/compare/v0.1.128...v0.1.129) (2026-04-21)


### Bug Fixes

* **ci:** add fastlane match provisioning for macOS desktop build ([#578](https://github.com/cedricziel/assistant/issues/578)) ([f85372b](https://github.com/cedricziel/assistant/commit/f85372b59a6d0232aacf5d60436cecd1556d25eb))

## [0.1.128](https://github.com/cedricziel/assistant/compare/v0.1.127...v0.1.128) (2026-04-20)


### Features

* **app:** adaptive streaming timeline widgets ([#568](https://github.com/cedricziel/assistant/issues/568)) ([fc01bf8](https://github.com/cedricziel/assistant/commit/fc01bf8003032af7db59eb92199e415547a1c0c6))
* **app:** add connectivity_plus for network awareness ([#562](https://github.com/cedricziel/assistant/issues/562)) ([be5c72a](https://github.com/cedricziel/assistant/commit/be5c72add9aeb3b0d7982836e329a05d7cd6e517))
* **app:** add outward sharing and save-as for messages, images, and audio ([#565](https://github.com/cedricziel/assistant/issues/565)) ([575c607](https://github.com/cedricziel/assistant/commit/575c607eaa02e951bdc1df46e167c9524efeb8cb))
* **runtime:** autonomous skill learning and self-improvement ([#571](https://github.com/cedricziel/assistant/issues/571)) ([a275be0](https://github.com/cedricziel/assistant/commit/a275be05c6009232d170478cf3eac9a78ec0246a))
* **runtime:** stream thinking, tool calls, and subagent events to clients ([#567](https://github.com/cedricziel/assistant/issues/567)) ([e01091e](https://github.com/cedricziel/assistant/commit/e01091eddf2a2ec36380c0072d56497bc4e2ba69))
* **web-ui,app:** throttle thinking persistence and handle cancelled subagents ([#570](https://github.com/cedricziel/assistant/issues/570)) ([0c9e0f2](https://github.com/cedricziel/assistant/commit/0c9e0f273d433ebf5b05de2547f08a27f8faf735))


### Bug Fixes

* **app:** update macOS code signing and add missing connectivity_plus pod ([#569](https://github.com/cedricziel/assistant/issues/569)) ([82f82e5](https://github.com/cedricziel/assistant/commit/82f82e51578143605fae03cdd2e448e9e253c2c8))
* **release:** add ShareExtension provisioning profile to match ([#564](https://github.com/cedricziel/assistant/issues/564)) ([0580c64](https://github.com/cedricziel/assistant/commit/0580c64432dd4c6c51da9c3ad333db3d11d66b20))
* **release:** bump macOS deployment target to 26.0 ([#566](https://github.com/cedricziel/assistant/issues/566)) ([c585fa5](https://github.com/cedricziel/assistant/commit/c585fa57879cc3fdcf0cf1a3c0ff3e3e55a1501c))

## [0.1.127](https://github.com/cedricziel/assistant/compare/v0.1.126...v0.1.127) (2026-04-20)


### Bug Fixes

* **release:** sign PlugIns/*.appex for macOS notarization ([#560](https://github.com/cedricziel/assistant/issues/560)) ([7a325f2](https://github.com/cedricziel/assistant/commit/7a325f272b916e0c2a0c61d028f912a3e867171c))

## [0.1.126](https://github.com/cedricziel/assistant/compare/v0.1.125...v0.1.126) (2026-04-20)


### Features

* expand attachment support and add native share extensions ([#552](https://github.com/cedricziel/assistant/issues/552)) ([abb6586](https://github.com/cedricziel/assistant/commit/abb6586200dfe6f673e65af18a04f1fe1d923271))
* reactive conversation list via SSE streaming ([#546](https://github.com/cedricziel/assistant/issues/546)) ([d1a1c6f](https://github.com/cedricziel/assistant/commit/d1a1c6f7240baab01c9dd080503a39c5a78f02b9))

## [0.1.125](https://github.com/cedricziel/assistant/compare/v0.1.124...v0.1.125) (2026-04-20)


### Bug Fixes

* **app:** remove dart:io Platform crash on web, add error screen ([#555](https://github.com/cedricziel/assistant/issues/555)) ([43a8358](https://github.com/cedricziel/assistant/commit/43a83585ba334f061ed39a9325abbb265e0457a1))
* **release:** include openapi.json in release-please version bumps ([#557](https://github.com/cedricziel/assistant/issues/557)) ([e47a3bf](https://github.com/cedricziel/assistant/commit/e47a3bf374ccd5fff98a4b05bb64ee6ea5c4b480))

## [0.1.124](https://github.com/cedricziel/assistant/compare/v0.1.123...v0.1.124) (2026-04-19)


### Features

* **app:** silent SSE stream reconnection on iOS background/resume ([#547](https://github.com/cedricziel/assistant/issues/547)) ([b7aa3c7](https://github.com/cedricziel/assistant/commit/b7aa3c7401603f28e71a5edf89892a88028f7437))
* **app:** slash-command autocomplete and timeline integration ([#550](https://github.com/cedricziel/assistant/issues/550)) ([132e5d7](https://github.com/cedricziel/assistant/commit/132e5d77f6e4ede1db29ce80d6d160e0fd634521))
* **shortcuts:** Apple Shortcuts integration via shared Swift Package ([#545](https://github.com/cedricziel/assistant/issues/545)) ([72512db](https://github.com/cedricziel/assistant/commit/72512db067b3a69c4bb6e7c61fe13069ec129fb2))
* **slack:** restore mention/thread-based message filtering ([#549](https://github.com/cedricziel/assistant/issues/549)) ([7413439](https://github.com/cedricziel/assistant/commit/7413439c086403620ef59bcac1ef31ecc8ba64e9))
* unified slash-command system across all interfaces ([#540](https://github.com/cedricziel/assistant/issues/540)) ([8be6acd](https://github.com/cedricziel/assistant/commit/8be6acd9d48c5820e341367548b76bb0ca43d6e5))

## [0.1.123](https://github.com/cedricziel/assistant/compare/v0.1.122...v0.1.123) (2026-04-19)


### Features

* **app:** adaptive widget polish — Phase 4 Apple-native UX ([#541](https://github.com/cedricziel/assistant/issues/541)) ([064e981](https://github.com/cedricziel/assistant/commit/064e981bf07782a000763965510bd35654a6b060))

## [0.1.122](https://github.com/cedricziel/assistant/compare/v0.1.121...v0.1.122) (2026-04-19)


### Features

* **app:** Cupertino page chrome — large titles and adaptive nav bars ([#538](https://github.com/cedricziel/assistant/issues/538)) ([7dbc868](https://github.com/cedricziel/assistant/commit/7dbc8680381a42dcf7eac424688b7035c7365f4f))
* **app:** CupertinoTabBar and sidebar navigation for iOS/iPadOS ([#535](https://github.com/cedricziel/assistant/issues/535)) ([83e1c28](https://github.com/cedricziel/assistant/commit/83e1c2833cb243ae72ef27d0d4b8797f612c728f))


### Bug Fixes

* **app:** convert HEIC images to PNG before upload on iOS ([#539](https://github.com/cedricziel/assistant/issues/539)) ([1d4f971](https://github.com/cedricziel/assistant/commit/1d4f97190cc2abd06dd4e579f4e59c2cd7807752))
* **app:** pass MIME type to audioplayers BytesSource on iOS ([#537](https://github.com/cedricziel/assistant/issues/537)) ([8f3c458](https://github.com/cedricziel/assistant/commit/8f3c458bde1fc678ec61d00074fee4088a240bda))

## [0.1.121](https://github.com/cedricziel/assistant/compare/v0.1.120...v0.1.121) (2026-04-19)


### Features

* **app:** add Siri and Action Button integration ([#532](https://github.com/cedricziel/assistant/issues/532)) ([a057403](https://github.com/cedricziel/assistant/commit/a057403b9b3fa6138a54bd7de7fe6c0a23e38e94))
* **app:** Apple-native UX foundation — Phase 1 ([#531](https://github.com/cedricziel/assistant/issues/531)) ([3438693](https://github.com/cedricziel/assistant/commit/34386930966dce8d386d3ead5e83ee3ddbed152a))


### Bug Fixes

* **app:** add MaterialLocalizations to CupertinoApp shell ([#534](https://github.com/cedricziel/assistant/issues/534)) ([15d01eb](https://github.com/cedricziel/assistant/commit/15d01ebf7b284a7b1489603b4dc05e853af6ee29))

## [0.1.120](https://github.com/cedricziel/assistant/compare/v0.1.119...v0.1.120) (2026-04-18)


### Bug Fixes

* **ci:** configure manual code signing for iOS archive in CI ([#528](https://github.com/cedricziel/assistant/issues/528)) ([c1cfe76](https://github.com/cedricziel/assistant/commit/c1cfe766774955c56374a15596462521d8181042))
* **web-ui:** vendor swagger-ui assets to avoid CI download failures ([#527](https://github.com/cedricziel/assistant/issues/527)) ([205c125](https://github.com/cedricziel/assistant/commit/205c125b0ec04194009e85678ff0d34bc8cb590d))

## [0.1.119](https://github.com/cedricziel/assistant/compare/v0.1.118...v0.1.119) (2026-04-18)


### Features

* chat timeline sections for thinking, tool calls, and subagents ([#523](https://github.com/cedricziel/assistant/issues/523)) ([c0eb280](https://github.com/cedricziel/assistant/commit/c0eb280d4c0d5e06a2e1d1950ee9275140554f10))


### Bug Fixes

* **ci:** use macos-26 runner for iOS builds ([#526](https://github.com/cedricziel/assistant/issues/526)) ([d36ec04](https://github.com/cedricziel/assistant/commit/d36ec04abc41e54f56f2da5d118b7b233cff42ed))
* **updater:** disable self-update feature on iOS to prevent crash ([#501](https://github.com/cedricziel/assistant/issues/501)) ([a557757](https://github.com/cedricziel/assistant/commit/a557757fdbe820eb55e2e8b0e3851a63160f2e10)), closes [#420](https://github.com/cedricziel/assistant/issues/420)

## [0.1.118](https://github.com/cedricziel/assistant/compare/v0.1.117...v0.1.118) (2026-04-18)


### Bug Fixes

* **ci:** add cocoapods to Gemfile for iOS TestFlight builds ([#521](https://github.com/cedricziel/assistant/issues/521)) ([3c0132e](https://github.com/cedricziel/assistant/commit/3c0132e9d5425c08303b7dd2c1f1625f9c03775c))

## [0.1.117](https://github.com/cedricziel/assistant/compare/v0.1.116...v0.1.117) (2026-04-18)


### Bug Fixes

* **ci:** reinstall CocoaPods with correct Ruby version in iOS TestFlight job ([#518](https://github.com/cedricziel/assistant/issues/518)) ([3f7cfb3](https://github.com/cedricziel/assistant/commit/3f7cfb3914487e4084ce563214d6d20df8ce97cc))

## [0.1.116](https://github.com/cedricziel/assistant/compare/v0.1.115...v0.1.116) (2026-04-18)


### Bug Fixes

* gate PwaUpdateListener to web-only ([#502](https://github.com/cedricziel/assistant/issues/502)) ([e6287fc](https://github.com/cedricziel/assistant/commit/e6287fc268819e6ef03b62944e104cd2607ecf38)), closes [#419](https://github.com/cedricziel/assistant/issues/419)

## [0.1.115](https://github.com/cedricziel/assistant/compare/v0.1.114...v0.1.115) (2026-04-18)


### Features

* **chat:** render user voice messages as mini audio player ([#514](https://github.com/cedricziel/assistant/issues/514)) ([7f506cd](https://github.com/cedricziel/assistant/commit/7f506cdaaf21c36388c6c3522e4a104f8303356d))
* **fastlane:** iOS release automation setup ([#418](https://github.com/cedricziel/assistant/issues/418)) ([43085a8](https://github.com/cedricziel/assistant/commit/43085a80db62c2c737e40750feb2c28476e7329b))
* **tts:** auto-detect language for Deepgram TTS voice selection ([#515](https://github.com/cedricziel/assistant/issues/515)) ([3c54e07](https://github.com/cedricziel/assistant/commit/3c54e07b9dc35291c07e2e34c6390d7917fc1e80))


### Bug Fixes

* **chat:** fix inline image 401 by watching auth profile reactively ([#512](https://github.com/cedricziel/assistant/issues/512)) ([4196b80](https://github.com/cedricziel/assistant/commit/4196b80b7a95efdae62f096a180832a85ab9e1dd))
* **transcription:** use valid Deepgram TTS model name aura-2-thalia-en ([#507](https://github.com/cedricziel/assistant/issues/507)) ([c7f8f78](https://github.com/cedricziel/assistant/commit/c7f8f7840c92d2b3bd962059d021f596154873bf))
* **workflow:** await task handle after abort to fix flaky test ([#516](https://github.com/cedricziel/assistant/issues/516)) ([46236de](https://github.com/cedricziel/assistant/commit/46236deb1f27688f061be2427bdc11970300aef2))

## [0.1.114](https://github.com/cedricziel/assistant/compare/v0.1.113...v0.1.114) (2026-04-18)


### Bug Fixes

* **app:** animate streaming dots indicator ([#508](https://github.com/cedricziel/assistant/issues/508)) ([7e80f49](https://github.com/cedricziel/assistant/commit/7e80f49492a1ce3676d33a2a3be9a3ccbf4fa9af))
* **app:** show image thumbnails immediately when sending ([#509](https://github.com/cedricziel/assistant/issues/509)) ([d253f46](https://github.com/cedricziel/assistant/commit/d253f468332dbfeb856aea251c1b5779ec55cbbe))
* **router:** preserve deep-link destination during async auth loading ([#511](https://github.com/cedricziel/assistant/issues/511)) ([65e3258](https://github.com/cedricziel/assistant/commit/65e325848ebc2358834068a36306fef616bf3bb4))

## [0.1.113](https://github.com/cedricziel/assistant/compare/v0.1.112...v0.1.113) (2026-04-17)


### Features

* **matrix:** add voice message support ([#488](https://github.com/cedricziel/assistant/issues/488)) ([0b58f2e](https://github.com/cedricziel/assistant/commit/0b58f2e64492d20d60b6c92eefe07a628bdcd516))


### Bug Fixes

* **analytics:** replace hardcoded colors with theme-aware equivalents ([#503](https://github.com/cedricziel/assistant/issues/503)) ([558e944](https://github.com/cedricziel/assistant/commit/558e944e86996e593895ff3924cf1f390332c1f0))
* **cli:** wire AudioStore into orchestrator for outbound audio delivery ([#505](https://github.com/cedricziel/assistant/issues/505)) ([47884a9](https://github.com/cedricziel/assistant/commit/47884a9999c359bc289fb53a020ff5a8852c0de4))
* **connection:** add isIOSPlatform helper and document iOS platform guards ([#504](https://github.com/cedricziel/assistant/issues/504)) ([652affe](https://github.com/cedricziel/assistant/commit/652affe58c9294938aa6f5d4c4532321e8d26686))
* **matrix:** collapse nested if-let to satisfy clippy ([#506](https://github.com/cedricziel/assistant/issues/506)) ([a1ad2dd](https://github.com/cedricziel/assistant/commit/a1ad2dd8b8fcd1db763f222860bf22836b575476))
* **notifications:** add iOS notification permission request and badge support ([#500](https://github.com/cedricziel/assistant/issues/500)) ([20393f7](https://github.com/cedricziel/assistant/commit/20393f75242ecf7b4b3259dca1815379ad76452b)), closes [#421](https://github.com/cedricziel/assistant/issues/421)

## [0.1.112](https://github.com/cedricziel/assistant/compare/v0.1.111...v0.1.112) (2026-04-17)


### Features

* **audio:** end-to-end audio pipeline across all messaging platforms ([#498](https://github.com/cedricziel/assistant/issues/498)) ([7a122a3](https://github.com/cedricziel/assistant/commit/7a122a3373bd06607b7d16e2ef60b601d963d53b))
* migrate workspace to Rust edition 2024 ([#495](https://github.com/cedricziel/assistant/issues/495)) ([7b6c674](https://github.com/cedricziel/assistant/commit/7b6c674a0def1700e95ebdd5954c7d3e4b51f1d4))


### Bug Fixes

* **runtime:** thread attachment_ids through run_turn* methods ([#497](https://github.com/cedricziel/assistant/issues/497)) ([56710b5](https://github.com/cedricziel/assistant/commit/56710b5ded4069c0ca0e95657df6cee7028c266a))

## [0.1.111](https://github.com/cedricziel/assistant/compare/v0.1.110...v0.1.111) (2026-04-17)


### Bug Fixes

* **app:** remove unresolved keychain-access-groups blocking macOS launch ([#491](https://github.com/cedricziel/assistant/issues/491)) ([c6845dc](https://github.com/cedricziel/assistant/commit/c6845dc77f514f6b475b6505577abef31f20b476))
* **hooks:** make pre-commit hook work in git worktrees ([#493](https://github.com/cedricziel/assistant/issues/493)) ([f17cf2f](https://github.com/cedricziel/assistant/commit/f17cf2f3f508fd0145ec34a18f08324d3731c1e3))


### Performance Improvements

* **ci:** speed up release pipeline with thin LTO and dedup macOS build ([#490](https://github.com/cedricziel/assistant/issues/490)) ([25202ae](https://github.com/cedricziel/assistant/commit/25202ae952b8f2a00bd1343606befae983441bda))

## [0.1.110](https://github.com/cedricziel/assistant/compare/v0.1.109...v0.1.110) (2026-04-16)


### Features

* **chat:** add meta action row and fix Deepgram TTS auth ([#487](https://github.com/cedricziel/assistant/issues/487)) ([98b2242](https://github.com/cedricziel/assistant/commit/98b2242f31873f116fc4465e6c306f3c4290b4d9))


### Bug Fixes

* **hooks:** warm Flutter deps before dart_pre_commit ([#486](https://github.com/cedricziel/assistant/issues/486)) ([a3a7a06](https://github.com/cedricziel/assistant/commit/a3a7a065a75618f1f91ee2ddfb62ca6b5fd55124))
* **images:** three bugs breaking image attachment flow ([#484](https://github.com/cedricziel/assistant/issues/484)) ([f7ae264](https://github.com/cedricziel/assistant/commit/f7ae26411627081730b59e8daaa57a2a9fdccabc))

## [0.1.109](https://github.com/cedricziel/assistant/compare/v0.1.108...v0.1.109) (2026-04-16)


### Bug Fixes

* **hooks:** use flutter pub run for dart_pre_commit ([#483](https://github.com/cedricziel/assistant/issues/483)) ([14730ff](https://github.com/cedricziel/assistant/commit/14730ffd4ab907c2eb40a6db676b942519a565be))
* **macos:** add file picker entitlement for sandboxed app ([#481](https://github.com/cedricziel/assistant/issues/481)) ([99b8cc8](https://github.com/cedricziel/assistant/commit/99b8cc8c5f6ee1e6ffc0041e4e247887cc001493))

## [0.1.108](https://github.com/cedricziel/assistant/compare/v0.1.107...v0.1.108) (2026-04-16)


### Features

* bidirectional image attachment support ([#479](https://github.com/cedricziel/assistant/issues/479)) ([cb492e7](https://github.com/cedricziel/assistant/commit/cb492e76aa6e1afc3243df23452cd439c7f2c1d0))

## [0.1.107](https://github.com/cedricziel/assistant/compare/v0.1.106...v0.1.107) (2026-04-16)


### Features

* **chat:** render mermaid diagrams and streaming markdown natively ([#477](https://github.com/cedricziel/assistant/issues/477)) ([15ed92e](https://github.com/cedricziel/assistant/commit/15ed92e1ecad3f25d4471b952a961392843429ed))
* **hooks:** add Flutter quality checks to pre-commit hook ([#476](https://github.com/cedricziel/assistant/issues/476)) ([09d5fc1](https://github.com/cedricziel/assistant/commit/09d5fc1818c71263eee20703039b3309226cbfdd))


### Bug Fixes

* **ci:** use ditto instead of zip to preserve macOS framework symlinks ([#475](https://github.com/cedricziel/assistant/issues/475)) ([a38a035](https://github.com/cedricziel/assistant/commit/a38a035b01888e156f4591b029d158dd46a9a714))

## [0.1.106](https://github.com/cedricziel/assistant/compare/v0.1.105...v0.1.106) (2026-04-16)


### Bug Fixes

* **chat:** gate audio button on per-message ttsAvailable flag ([#473](https://github.com/cedricziel/assistant/issues/473)) ([664a5f5](https://github.com/cedricziel/assistant/commit/664a5f5a8031e58dc1c289c719010b1a1fd8865d))

## [0.1.105](https://github.com/cedricziel/assistant/compare/v0.1.104...v0.1.105) (2026-04-16)


### Features

* **chat:** include tool call arguments and results in UI and API ([#471](https://github.com/cedricziel/assistant/issues/471)) ([b3a2b53](https://github.com/cedricziel/assistant/commit/b3a2b53ca0eab7c4f070c5c57fb7acad14824b50))

## [0.1.104](https://github.com/cedricziel/assistant/compare/v0.1.103...v0.1.104) (2026-04-16)


### Features

* durable conversation event log with SSE replay ([#469](https://github.com/cedricziel/assistant/issues/469)) ([afad95e](https://github.com/cedricziel/assistant/commit/afad95eff6bfc379e0fd1c209f30078f0c7b4cea))

## [0.1.103](https://github.com/cedricziel/assistant/compare/v0.1.102...v0.1.103) (2026-04-16)


### Features

* **chat:** add tts_available field to MessageSummary for accurate audio button visibility ([#467](https://github.com/cedricziel/assistant/issues/467)) ([af6ac56](https://github.com/cedricziel/assistant/commit/af6ac5677f7ca9f00416e6322da0bfb31c1284db))


### Bug Fixes

* **app:** repair web voice audio bugs — transcript bubble and play button 400 ([#464](https://github.com/cedricziel/assistant/issues/464)) ([7c64076](https://github.com/cedricziel/assistant/commit/7c6407623103652e989f4af5c230eeec770ba54e))

## [0.1.102](https://github.com/cedricziel/assistant/compare/v0.1.101...v0.1.102) (2026-04-16)


### Features

* **app:** render tool call chips inline in assistant message bubbles ([#465](https://github.com/cedricziel/assistant/issues/465)) ([98280b5](https://github.com/cedricziel/assistant/commit/98280b57f73679efa4ae861c9a226192de83ce9e))


### Bug Fixes

* **app:** fix provider race condition on page open + widget tests ([#463](https://github.com/cedricziel/assistant/issues/463)) ([06fb0ef](https://github.com/cedricziel/assistant/commit/06fb0ef41a06c2415a4c6bd447c63fc01a853e54))

## [0.1.101](https://github.com/cedricziel/assistant/compare/v0.1.100...v0.1.101) (2026-04-15)


### Bug Fixes

* **app:** use generated CapabilitiesApi — voice buttons now visible on web/PWA ([#461](https://github.com/cedricziel/assistant/issues/461)) ([b10647e](https://github.com/cedricziel/assistant/commit/b10647ee0a63fd7fdfff9513cdefb500130ca8f2))

## [0.1.100](https://github.com/cedricziel/assistant/compare/v0.1.99...v0.1.100) (2026-04-15)


### Features

* **interfaces:** propose working-signals change (hourglass queue + typing indicators + Slack setStatus) ([#456](https://github.com/cedricziel/assistant/issues/456)) ([34e25ab](https://github.com/cedricziel/assistant/commit/34e25ab66525d2bdc5470de4c04021815030d9c9))

## [0.1.99](https://github.com/cedricziel/assistant/compare/v0.1.98...v0.1.99) (2026-04-15)


### Features

* **web-ui:** replace default Flutter PWA icons with app icon ([#452](https://github.com/cedricziel/assistant/issues/452)) ([a5a2b83](https://github.com/cedricziel/assistant/commit/a5a2b8326bc0b2fe3d4fa6cc23d29598ccfff6cd)), closes [#443](https://github.com/cedricziel/assistant/issues/443)
* **web-ui:** voice messages — mic send (STT) + audio playback (TTS) ([#451](https://github.com/cedricziel/assistant/issues/451)) ([5b1c3e1](https://github.com/cedricziel/assistant/commit/5b1c3e1770af147d1ac1886c8866796bb813bbce))


### Bug Fixes

* **runtime:** increase default soft_threshold to 30k to prevent near-limit compaction miss ([#458](https://github.com/cedricziel/assistant/issues/458)) ([a2d415a](https://github.com/cedricziel/assistant/commit/a2d415a1911f576e2737306c6b6355e32ed12d3e))

## [0.1.98](https://github.com/cedricziel/assistant/compare/v0.1.97...v0.1.98) (2026-04-15)


### Features

* **app:** auto-scroll to latest message and add scroll-to-bottom FAB ([#450](https://github.com/cedricziel/assistant/issues/450)) ([2b94e7c](https://github.com/cedricziel/assistant/commit/2b94e7c138944e71a1bbc07a41d8c149d5e05fda))
* **skills:** add openapi-sync skill to enforce API spec discipline ([3b05268](https://github.com/cedricziel/assistant/commit/3b052687d35e3d99c0a3d1c32c687940f4e2da2c))


### Bug Fixes

* **app:** scroll chat to bottom when keyboard opens on iOS Safari ([#448](https://github.com/cedricziel/assistant/issues/448)) ([9644890](https://github.com/cedricziel/assistant/commit/96448906bd59ff97cf0ce86d5e615d0d7bc77c95))

## [0.1.97](https://github.com/cedricziel/assistant/compare/v0.1.96...v0.1.97) (2026-04-14)


### Features

* **web-ui:** voice message send and receive ([#434](https://github.com/cedricziel/assistant/issues/434)) ([8e8ae35](https://github.com/cedricziel/assistant/commit/8e8ae35166033c6e2d6a1d87b1941329c9f2479a))

## [0.1.96](https://github.com/cedricziel/assistant/compare/v0.1.95...v0.1.96) (2026-04-14)


### Features

* **runtime:** route scheduler output to persona home channel ([#438](https://github.com/cedricziel/assistant/issues/438)) ([aaf92ec](https://github.com/cedricziel/assistant/commit/aaf92ec0fe9fb429d496fc617007e1ccb08bb728))


### Bug Fixes

* **app:** persist web session across hard reloads ([#440](https://github.com/cedricziel/assistant/issues/440)) ([4c9e28b](https://github.com/cedricziel/assistant/commit/4c9e28b6c6cb523fbdd827f8e3cf511c3830496b))
* **interface-cli:** wire transcription provider to Matrix interface ([291111b](https://github.com/cedricziel/assistant/commit/291111bfc72a24c01430d2f4c0941120db85fe8c))
* **pwa:** Safari install prompt and update detection ([#439](https://github.com/cedricziel/assistant/issues/439)) ([8709326](https://github.com/cedricziel/assistant/commit/870932672dfe6dac8e2e3e8d1b97c2f094638749))
* **release:** fix macOS build signing and sandbox for CI ([#436](https://github.com/cedricziel/assistant/issues/436)) ([17029f7](https://github.com/cedricziel/assistant/commit/17029f772cf63f8c203379e61e0f01b33e6bdea9))

## [0.1.95](https://github.com/cedricziel/assistant/compare/v0.1.94...v0.1.95) (2026-04-14)


### Features

* **app:** web login screen with auto-context and logout ([#432](https://github.com/cedricziel/assistant/issues/432)) ([b6ec140](https://github.com/cedricziel/assistant/commit/b6ec140a72a2458cb6fff76dd0a4ca0020fccca2))
* **interface-matrix:** voice and image message support ([#433](https://github.com/cedricziel/assistant/issues/433)) ([8109dda](https://github.com/cedricziel/assistant/commit/8109dda54b2ab47932b5d75bc2da0c3f744a2689))

## [0.1.94](https://github.com/cedricziel/assistant/compare/v0.1.93...v0.1.94) (2026-04-14)


### Features

* **app:** chat message queue and retry ([#424](https://github.com/cedricziel/assistant/issues/424)) ([9e645ee](https://github.com/cedricziel/assistant/commit/9e645eede888d46a4c06099bea8e10a0b742a16d))


### Bug Fixes

* **app:** disable codesign in flutter CI, update macOS bundle ID ([013d2f7](https://github.com/cedricziel/assistant/commit/013d2f7a6475cac9ecba82f29de31c538a6652a2))

## [0.1.93](https://github.com/cedricziel/assistant/compare/v0.1.92...v0.1.93) (2026-04-14)


### Features

* **app:** add context switcher for multi-server support ([cbaa293](https://github.com/cedricziel/assistant/commit/cbaa293dc6b88c4e454159f7be419b561ca126b6))
* **app:** iOS/iPadOS remote-only app support ([#417](https://github.com/cedricziel/assistant/issues/417)) ([203ef6b](https://github.com/cedricziel/assistant/commit/203ef6b16290286b3aaedaa87ec3e559d2669a75))
* **app:** move Contexts to NavigationRail trailing slot ([6676b26](https://github.com/cedricziel/assistant/commit/6676b26db18528e0a085c350bcbfd949e09b7255))
* **app:** move Settings to sticky trailing slot with separator ([e81b072](https://github.com/cedricziel/assistant/commit/e81b07242c3bdba12ff4a97cc4639aa0caf4fac8))


### Bug Fixes

* **app:** make trailing nav section truly sticky by moving outside scroll area ([b3289bd](https://github.com/cedricziel/assistant/commit/b3289bd61b4d7310f7cbdb0c315d36320a4c1809))
* **app:** prevent NavigationRail overflow on short screens ([f986918](https://github.com/cedricziel/assistant/commit/f98691816f1f4ea22011d193c9ed6953385769cf))
* **app:** resolve macOS keychain -34018 error with proper signing setup ([#414](https://github.com/cedricziel/assistant/issues/414)) ([1f40b27](https://github.com/cedricziel/assistant/commit/1f40b27d90bc965f31257cb96a8791d437f0e281))

## [0.1.92](https://github.com/cedricziel/assistant/compare/v0.1.91...v0.1.92) (2026-04-10)


### Bug Fixes

* **ci:** sign embedded Rust binary before notarization ([25697a3](https://github.com/cedricziel/assistant/commit/25697a32ef42665c4f8ba5178a4b1df99a78c75f))

## [0.1.91](https://github.com/cedricziel/assistant/compare/v0.1.90...v0.1.91) (2026-04-10)


### Features

* **app:** mobile-friendly nav with "More" overflow sheet ([#404](https://github.com/cedricziel/assistant/issues/404)) ([68a0d82](https://github.com/cedricziel/assistant/commit/68a0d82bda6531923f5f2d26a734587e8fc90b4b))
* **app:** replace flutter_markdown with flutter_markdown_plus ([#406](https://github.com/cedricziel/assistant/issues/406)) ([ba4d55e](https://github.com/cedricziel/assistant/commit/ba4d55ee4a20b7a07cabc3b5f77d3a371709b2a9))

## [0.1.90](https://github.com/cedricziel/assistant/compare/v0.1.89...v0.1.90) (2026-04-09)


### Features

* **app:** push notifications via VAPID Web Push and flutter_local_notifications ([#402](https://github.com/cedricziel/assistant/issues/402)) ([b596c5e](https://github.com/cedricziel/assistant/commit/b596c5efe32fb685712865f5541b93c21f582280))

## [0.1.89](https://github.com/cedricziel/assistant/compare/v0.1.88...v0.1.89) (2026-04-08)


### Features

* **app:** PWA install prompt ([#400](https://github.com/cedricziel/assistant/issues/400)) ([8ef9606](https://github.com/cedricziel/assistant/commit/8ef9606cb8c04bcdfbf26dbc48d9b7c396670a29))

## [0.1.88](https://github.com/cedricziel/assistant/compare/v0.1.87...v0.1.88) (2026-04-08)


### Features

* comprehensive SSE event streaming for tool calls and turn lifecycle ([a207e37](https://github.com/cedricziel/assistant/commit/a207e375ef1423655087a7963c77c105864eeb56))

## [0.1.87](https://github.com/cedricziel/assistant/compare/v0.1.86...v0.1.87) (2026-04-08)


### Features

* **chat:** render assistant messages as markdown ([eaca94b](https://github.com/cedricziel/assistant/commit/eaca94b5856254da86b7c3070763fe34c4cb35cb))

## [0.1.86](https://github.com/cedricziel/assistant/compare/v0.1.85...v0.1.86) (2026-04-08)


### Features

* **app:** add skill CRUD, workflow CRUD, trace detail, and workflow run detail screens ([10574fe](https://github.com/cedricziel/assistant/commit/10574fea67b87f0a7d009eb45a96a00c1734e04e))


### Bug Fixes

* **web-ui:** apply empty_string_as_none to AnalyticsQueryParams.window ([3899b2b](https://github.com/cedricziel/assistant/commit/3899b2bedffba029478763175b16f235dd2b09bc))
* **web-ui:** tolerate empty-string query params from generated Dart client ([082eeb8](https://github.com/cedricziel/assistant/commit/082eeb8633795a6ea02e1ffda412c492b39c3ec7))

## [0.1.85](https://github.com/cedricziel/assistant/compare/v0.1.84...v0.1.85) (2026-04-07)


### Features

* **personas:** add search/filter to personas list ([122b44e](https://github.com/cedricziel/assistant/commit/122b44e3237bf721ff0817b018b9e63679ea75b2))


### Bug Fixes

* **agents:** guard card parsing with try/catch and fall back to agent ID ([7e01569](https://github.com/cedricziel/assistant/commit/7e01569b8cd767491ed51796b456b0e8cfc48b51))
* **agents:** show skills list from agent card in detail screen ([ffde799](https://github.com/cedricziel/assistant/commit/ffde7993f56a75d0d4e447f0f7bf75adac627423))
* **analytics:** wrap DataTables in horizontal scroll to prevent mobile overflow ([63021fa](https://github.com/cedricziel/assistant/commit/63021fa1cbc5beb9c48cbfb612f559c44e90a6dd))
* **chat:** reduce spacing between consecutive messages from the same sender ([01d4158](https://github.com/cedricziel/assistant/commit/01d4158a043a4ee1d06c48ca0a350dd4e66ffcd5))
* **connection:** add token visibility toggle to authentication field ([eba3d8c](https://github.com/cedricziel/assistant/commit/eba3d8cdad344585401087264cbfc1987d43fc23))
* **logs:** show spinner in search field during debounce window ([4391dfa](https://github.com/cedricziel/assistant/commit/4391dfa53361f611458da40558a4de761b0155f2))
* **logs:** truncate long messages to 2 lines, tap to expand ([c53fcfc](https://github.com/cedricziel/assistant/commit/c53fcfc06ec4897691ce414da80337484278ca57))
* **personas:** improve file slot tile with displayName, amber missing badge, and tooltip ([348d068](https://github.com/cedricziel/assistant/commit/348d0683e58c5859502b4eed59f44c3f4054c461))
* **personas:** show success snackbar after creating a persona ([f0f6434](https://github.com/cedricziel/assistant/commit/f0f6434dfadfe3f5cd2e1097e235e14c40b20ac6))
* **personas:** use monospace font in file editor for readability ([4fafed2](https://github.com/cedricziel/assistant/commit/4fafed278bebe4bdf2c9c3444fe58b678197ae65))
* **skills:** allow description expand-on-tap in list with detail navigation icon ([92cddb6](https://github.com/cedricziel/assistant/commit/92cddb6917905be1689becc32612ceff6afbe861))
* **skills:** show checkmark icon for 2s after copying skill body ([cafa45a](https://github.com/cedricziel/assistant/commit/cafa45a4aff2bc2a112e8b4b626d5c8bd4009ce4))
* **traces:** preserve expanded row state across scroll by lifting to screen level ([88355b6](https://github.com/cedricziel/assistant/commit/88355b63853958666189b5d3805f4e37368d73c3))
* **traces:** reserve fixed height for span detail loading to prevent layout jank ([3bb40db](https://github.com/cedricziel/assistant/commit/3bb40db957b72d378397d57d5acf0668a7bcd9b5))
* **webhooks:** add tooltip explaining what webhook verification does ([f81fa3a](https://github.com/cedricziel/assistant/commit/f81fa3abafa32b3fcb92a91924495f07e14f36b3))
* **webhooks:** require confirmation dialog before rotating webhook secret ([cb96495](https://github.com/cedricziel/assistant/commit/cb9649524f855bf5fb89a430083596594ee0a16f))
* **workflows:** cap displayed runs at 20 with overflow count label ([1999062](https://github.com/cedricziel/assistant/commit/1999062b9056bd1fd42fd8725647cc0c7366603b))
* **workflows:** move active/inactive badge into title row to prevent clipping on narrow screens ([49d941e](https://github.com/cedricziel/assistant/commit/49d941e182a58d67576732be9c16f3a3b379e831))

## [0.1.84](https://github.com/cedricziel/assistant/compare/v0.1.83...v0.1.84) (2026-04-07)


### Features

* **chat:** add stop generation button to cancel in-progress streaming ([a01ef1e](https://github.com/cedricziel/assistant/commit/a01ef1ebac15913f0a6cd095810497121ad41346))
* **ios:** add iOS platform support with Neural Void icon ([#394](https://github.com/cedricziel/assistant/issues/394)) ([05fbf50](https://github.com/cedricziel/assistant/commit/05fbf50abc99d0851f0ccbd83f2f79035e70bdd7))


### Bug Fixes

* **agents:** add services import for Clipboard in agent detail ([1f2b763](https://github.com/cedricziel/assistant/commit/1f2b763a07c691cf1bc5657888e9edc46a332b45))
* **agents:** make agent URL copyable in detail screen ([18a6669](https://github.com/cedricziel/assistant/commit/18a6669f3263795892160109554eebf4e996c788))
* **app:** fix SSE stream type error on iOS (Utf8Decoder → utf8.decode) ([#392](https://github.com/cedricziel/assistant/issues/392)) ([2dff458](https://github.com/cedricziel/assistant/commit/2dff458e5d1f82fffae18bff307519b45e829ca6))
* **chat:** dismiss error banner without clearing conversation messages ([e7cd93d](https://github.com/cedricziel/assistant/commit/e7cd93d16d1d82de0d7e5cad45f2f1e9c84dbc4e))
* **chat:** dismiss keyboard when tapping outside the input field ([969cc59](https://github.com/cedricziel/assistant/commit/969cc59badeb1bb9bc407e661380ae2c0899447e))
* **chat:** require confirmation before deleting a conversation ([33d6e16](https://github.com/cedricziel/assistant/commit/33d6e16c59d83e3d8dcee206798917a6aa53c984))
* **chat:** respect iOS home indicator safe area in input row ([39b0ef9](https://github.com/cedricziel/assistant/commit/39b0ef9410949951f90e4b8ab05c2ff5f2e6ee19))
* **chat:** use fixed max-width for message bubbles instead of screen-width fraction ([e1770b6](https://github.com/cedricziel/assistant/commit/e1770b6e999a78aeaef71d25d4e9723d7d3079ff))
* **connection:** wrap setup screen body in SafeArea ([4ce4440](https://github.com/cedricziel/assistant/commit/4ce444071d495f6ed61336eec6bbf004d5bd6052))
* **logs:** dismiss keyboard when tapping outside the search field ([bd97ba1](https://github.com/cedricziel/assistant/commit/bd97ba169b749736133626def4f10765dbfeb5a0))
* **nav:** wrap wide navigation rail layout in SafeArea ([585aeb5](https://github.com/cedricziel/assistant/commit/585aeb5a364cffca6fe04cdef0a48cbbfa123189))
* **personas:** add SafeArea to file editor body ([7e34b93](https://github.com/cedricziel/assistant/commit/7e34b93ea7fc895e0e2f036e1e93ef5d37040338))
* **personas:** add SafeArea to persona create screen ([fa86b49](https://github.com/cedricziel/assistant/commit/fa86b49789a4aa2232530d044b38ffe157a4421e))
* **personas:** auto-focus ID field and add Cancel button on create screen ([e6a8799](https://github.com/cedricziel/assistant/commit/e6a8799ddb8e5cd852c20cc4b762d6c3c659a559))
* **personas:** warn before discarding unsaved file editor changes ([70b894b](https://github.com/cedricziel/assistant/commit/70b894b60e6d9c94bee226b6311fca0957022f87))
* **skills:** add copy button for skill body content ([d79125e](https://github.com/cedricziel/assistant/commit/d79125e2cfc3b4b3bfd55108747a493d701dd87f))
* **skills:** add missing services import for Clipboard ([44b54bb](https://github.com/cedricziel/assistant/commit/44b54bbbeae6332bb8033ab5d996977e2fc25f14))

## [0.1.83](https://github.com/cedricziel/assistant/compare/v0.1.82...v0.1.83) (2026-04-07)


### Bug Fixes

* **ci:** correct macOS app bundle name in zip command ([960c692](https://github.com/cedricziel/assistant/commit/960c692b994a695aa5654382926a1e616e15a1c9))
* commit podfile.lock ([0e56ce5](https://github.com/cedricziel/assistant/commit/0e56ce539f52225cc3f33d62a3d7b804c5d15b9e))
* **deps:** optimize Rust workspace dependencies ([#391](https://github.com/cedricziel/assistant/issues/391)) ([e0ddb34](https://github.com/cedricziel/assistant/commit/e0ddb3460fa8037ebd0b4fa130adce91d7274789))
* **embedded-server:** guard Platform.isMacOS with kIsWeb to prevent crash on web ([2790094](https://github.com/cedricziel/assistant/commit/27900940be394b509028174cfa605870f232e437))
* **embedded-server:** replace drain&lt;List&lt;int&gt;&gt;() with drain&lt;void&gt;() ([2f42645](https://github.com/cedricziel/assistant/commit/2f42645933af0e93944f46e5879b5db4dcb25d8b))
* **web:** add crossorigin=use-credentials to manifest link ([d9db26a](https://github.com/cedricziel/assistant/commit/d9db26a734e5e98b076082c4819786b83c2aa177))

## [0.1.82](https://github.com/cedricziel/assistant/compare/v0.1.81...v0.1.82) (2026-04-07)


### Bug Fixes

* **app:** sync Flutter app version with release-please and Rust workspace ([#388](https://github.com/cedricziel/assistant/issues/388)) ([ad1f91d](https://github.com/cedricziel/assistant/commit/ad1f91d16bc993bfdc34de2c9408e446b2bff15d))

## [0.1.81](https://github.com/cedricziel/assistant/compare/v0.1.80...v0.1.81) (2026-04-07)


### Features

* **app:** add macOS menu bar tray icon ([#385](https://github.com/cedricziel/assistant/issues/385)) ([5d3ea49](https://github.com/cedricziel/assistant/commit/5d3ea493d2f8d252cd925671562903a67dfc6c6b))
* **app:** Flutter desktop self-update via GitHub Releases ([#386](https://github.com/cedricziel/assistant/issues/386)) ([c740430](https://github.com/cedricziel/assistant/commit/c740430e97a1649c48a39cf79dbb159167ad40a9))
* Flutter cross-platform frontend (web + macOS) ([#374](https://github.com/cedricziel/assistant/issues/374)) ([ef7f09c](https://github.com/cedricziel/assistant/commit/ef7f09c29617364cef5cf5784703615aa48c559a))
* **flutter:** embed assistant binary in macOS app bundle ([#383](https://github.com/cedricziel/assistant/issues/383)) ([4a7aff8](https://github.com/cedricziel/assistant/commit/4a7aff8d45e1ea094fe823377a12cdf782a4bbad))
* migrate to Flutter-only UX with pure REST API backend ([#382](https://github.com/cedricziel/assistant/issues/382)) ([c2bc352](https://github.com/cedricziel/assistant/commit/c2bc352a8dd969fa189f62c24d480df02d8518a5))
* replace heavy SDK deps with thin reqwest/WS messenger clients ([#384](https://github.com/cedricziel/assistant/issues/384)) ([25f3ebf](https://github.com/cedricziel/assistant/commit/25f3ebfdeebe0bcbf9e1b4380cd7b30f0287e0b2))
* **web-ui:** replace HTMX/HTML layer with Flutter SPA frontend ([55449da](https://github.com/cedricziel/assistant/commit/55449da728000ec4b2b95cda4426ce5a8a71e340))


### Bug Fixes

* **ci:** correct release packaging for web UI embedding and macOS bundle ([#387](https://github.com/cedricziel/assistant/issues/387)) ([2912b81](https://github.com/cedricziel/assistant/commit/2912b81d6483a8675cfe5ff85b1e8ab344e02d92))

## [0.1.80](https://github.com/cedricziel/assistant/compare/v0.1.79...v0.1.80) (2026-04-04)


### Bug Fixes

* **interface-slack:** unblock WebSocket reader by spawning heavy work ([#372](https://github.com/cedricziel/assistant/issues/372)) ([9517197](https://github.com/cedricziel/assistant/commit/9517197454708ffa5e89435cbf0c50a3ad64cfab))

## [0.1.79](https://github.com/cedricziel/assistant/compare/v0.1.78...v0.1.79) (2026-04-04)


### Bug Fixes

* **iceberg-exporter:** pass partition key to fix partition validation error ([#369](https://github.com/cedricziel/assistant/issues/369)) ([ee55a04](https://github.com/cedricziel/assistant/commit/ee55a04cb3cd2ed87fbf85abf61f114094b70e86))

## [0.1.78](https://github.com/cedricziel/assistant/compare/v0.1.77...v0.1.78) (2026-04-03)


### Bug Fixes

* **ci:** pass --allow-untrusted to apk index to accept nfpm-built packages ([#367](https://github.com/cedricziel/assistant/issues/367)) ([a72bcf2](https://github.com/cedricziel/assistant/commit/a72bcf254d0189c1901061df9126ed687bc6a0f3))

## [0.1.77](https://github.com/cedricziel/assistant/compare/v0.1.76...v0.1.77) (2026-04-03)


### Features

* **skills:** add rust-dependencies and ci-organization skills ([#365](https://github.com/cedricziel/assistant/issues/365)) ([b48b998](https://github.com/cedricziel/assistant/commit/b48b99866344d9bdcab7420cb5f034e8e6134d99))


### Bug Fixes

* **interface-matrix:** vendor OpenSSL for rusqlite sqlcipher musl cross-build ([#363](https://github.com/cedricziel/assistant/issues/363)) ([0482b80](https://github.com/cedricziel/assistant/commit/0482b805f28d40b4a7582f1abcdc095b1dda4327))

## [0.1.76](https://github.com/cedricziel/assistant/compare/v0.1.75...v0.1.76) (2026-04-03)


### Features

* **interface-slack:** true incremental streaming via token sink ([#359](https://github.com/cedricziel/assistant/issues/359)) ([bd56fe2](https://github.com/cedricziel/assistant/commit/bd56fe25ab2ae124e83268a8c4784f5d71c60dcb))


### Bug Fixes

* **cross:** vendor OpenSSL + fix musl cross-compilation environment ([#362](https://github.com/cedricziel/assistant/issues/362)) ([b69e801](https://github.com/cedricziel/assistant/commit/b69e80179c5ceb7247a936204ccab9772af18fd0))

## [0.1.75](https://github.com/cedricziel/assistant/compare/v0.1.74...v0.1.75) (2026-04-03)


### Features

* **runtime:** cancel in-flight worker turn when submit_turn times out ([#358](https://github.com/cedricziel/assistant/issues/358)) ([f1d6d96](https://github.com/cedricziel/assistant/commit/f1d6d96e75af5fdbb55c01990e7be89f6a3f076f))


### Bug Fixes

* **interface-mattermost:** vendor OpenSSL to fix musl cross-compilation ([#357](https://github.com/cedricziel/assistant/issues/357)) ([dae7df4](https://github.com/cedricziel/assistant/commit/dae7df436a829b19e435dd1fe992496e8a7b794d))

## [0.1.74](https://github.com/cedricziel/assistant/compare/v0.1.73...v0.1.74) (2026-04-03)


### Features

* **runtime:** per-persona turn timeout (default 3 h) ([#355](https://github.com/cedricziel/assistant/issues/355)) ([adf8d4c](https://github.com/cedricziel/assistant/commit/adf8d4c64f18d38dbfa6f8787f98a4afbd41e97f))

## [0.1.73](https://github.com/cedricziel/assistant/compare/v0.1.72...v0.1.73) (2026-04-03)


### Features

* **release:** Alpine APK packages and lean Alpine Docker image ([#352](https://github.com/cedricziel/assistant/issues/352)) ([1b850b4](https://github.com/cedricziel/assistant/commit/1b850b4c2ac17cb7f8504ecba77aef2d40744595))


### Bug Fixes

* **iceberg-exporter:** use '+00:00' timezone to match Iceberg Timestamptz Arrow mapping ([#353](https://github.com/cedricziel/assistant/issues/353)) ([1a76570](https://github.com/cedricziel/assistant/commit/1a7657005c99fdcb813cc75ec5d77985c402d8a4))

## [0.1.72](https://github.com/cedricziel/assistant/compare/v0.1.71...v0.1.72) (2026-03-31)


### Features

* **interface-slack:** stream replies via chat.update placeholder ([#349](https://github.com/cedricziel/assistant/issues/349)) ([b5503db](https://github.com/cedricziel/assistant/commit/b5503db0ff07aa4fde8ba12552db37cd2e23284f))
* **web-ui:** JSON conversation API + Stimulus chat UI migration ([#348](https://github.com/cedricziel/assistant/issues/348)) ([9602bd8](https://github.com/cedricziel/assistant/commit/9602bd8808322b65e2324e7f23022b1a0b69a944))


### Bug Fixes

* **claude:** repair WorktreeCreate hook to correctly create worktrees ([#350](https://github.com/cedricziel/assistant/issues/350)) ([0d32d5c](https://github.com/cedricziel/assistant/commit/0d32d5c8c5f3f7333d695b70805b12e46ce62d38))

## [0.1.71](https://github.com/cedricziel/assistant/compare/v0.1.70...v0.1.71) (2026-03-30)


### Features

* **claude:** add WorktreeCreate hook for web-ui vendoring and shared Cargo target ([#346](https://github.com/cedricziel/assistant/issues/346)) ([683d757](https://github.com/cedricziel/assistant/commit/683d757fb85f4253b188e26247bf310d2c3845ae))
* **web-ui:** query Iceberg Parquet files when exporter = "iceberg" ([#345](https://github.com/cedricziel/assistant/issues/345)) ([f41dd56](https://github.com/cedricziel/assistant/commit/f41dd56c098318a55bddb0bc1bdb0174d027542b))

## [0.1.70](https://github.com/cedricziel/assistant/compare/v0.1.69...v0.1.70) (2026-03-30)


### Features

* **skill:** upgrade claude-code-agent to use native process tool (Phase 3 of [#74](https://github.com/cedricziel/assistant/issues/74)) ([cbec05a](https://github.com/cedricziel/assistant/commit/cbec05a60b75b8b4ef6aa2e5ce57a51c4699a6c1))


### Bug Fixes

* **iceberg-exporter:** derive Arrow schemas from Iceberg schema to embed field IDs ([#343](https://github.com/cedricziel/assistant/issues/343)) ([f7a31e9](https://github.com/cedricziel/assistant/commit/f7a31e99684dd0b488d2cc3332651eb64aed00a9))

## [0.1.69](https://github.com/cedricziel/assistant/compare/v0.1.68...v0.1.69) (2026-03-30)


### Features

* **web-ui:** expose OpenAPI spec and Swagger UI ([#339](https://github.com/cedricziel/assistant/issues/339)) ([9b98d42](https://github.com/cedricziel/assistant/commit/9b98d42cf7c26be6bdc575980fd132b2fdc404e2))

## [0.1.68](https://github.com/cedricziel/assistant/compare/v0.1.67...v0.1.68) (2026-03-30)


### Features

* **observability:** add Apache Iceberg exporter for OTel spans, logs, and metrics ([#338](https://github.com/cedricziel/assistant/issues/338)) ([cc15760](https://github.com/cedricziel/assistant/commit/cc157603db3650365fae8e29682555fb73e5bd4f))


### Bug Fixes

* correct opentelemetry-exporter-iceberg version ([8a1d483](https://github.com/cedricziel/assistant/commit/8a1d4832c6800e540f9a77047dedbf9ac97e34c8))

## [0.1.67](https://github.com/cedricziel/assistant/compare/v0.1.66...v0.1.67) (2026-03-29)


### Features

* **backup:** add backup and restore CLI subcommands ([#330](https://github.com/cedricziel/assistant/issues/330)) ([dec47de](https://github.com/cedricziel/assistant/commit/dec47de1a1289c34b8bf10b747a088e35a87a2fc))

## [0.1.66](https://github.com/cedricziel/assistant/compare/v0.1.65...v0.1.66) (2026-03-29)


### Bug Fixes

* **runtime:** raise OTel log bridge filter from DEBUG to INFO, suppress async_nats/h2/hyper_util ([f47e810](https://github.com/cedricziel/assistant/commit/f47e8102dc2db32ed75bab9a991f751153a68b23))

## [0.1.65](https://github.com/cedricziel/assistant/compare/v0.1.64...v0.1.65) (2026-03-29)


### Features

* **003-skill-management:** skill CRUD, persona access control, and AI generation ([#326](https://github.com/cedricziel/assistant/issues/326)) ([fa9d40a](https://github.com/cedricziel/assistant/commit/fa9d40af4b24c816b22f332704b0f50e9ccce447))

## [0.1.64](https://github.com/cedricziel/assistant/compare/v0.1.63...v0.1.64) (2026-03-29)


### Features

* **observability:** UI improvements for diagnosing silent failures ([#327](https://github.com/cedricziel/assistant/issues/327)) ([7428621](https://github.com/cedricziel/assistant/commit/74286216e7e9dc07431b6a8a5946ffb0f7d8269c))

## [0.1.63](https://github.com/cedricziel/assistant/compare/v0.1.62...v0.1.63) (2026-03-28)


### Features

* **interface-matrix:** add Matrix messaging interface ([#325](https://github.com/cedricziel/assistant/issues/325)) ([57198ca](https://github.com/cedricziel/assistant/commit/57198ca24041b8957fafc43532116d3c57326c7a))
* **web-ui:** persona file editor UI ([#323](https://github.com/cedricziel/assistant/issues/323)) ([f718966](https://github.com/cedricziel/assistant/commit/f718966b1fe3c27a960a43f6f420904ace0e963c))

## [0.1.62](https://github.com/cedricziel/assistant/compare/v0.1.61...v0.1.62) (2026-03-27)


### Bug Fixes

* **cli/runtime:** scheduled tasks silent in orchestrator interface-filtered mode ([#319](https://github.com/cedricziel/assistant/issues/319)) ([d3dc567](https://github.com/cedricziel/assistant/commit/d3dc567c64271d4a815a52516e74b6308c099329))

## [0.1.61](https://github.com/cedricziel/assistant/compare/v0.1.60...v0.1.61) (2026-03-27)


### Bug Fixes

* **cli:** spawn scheduler worker in single-interface modes ([3972b17](https://github.com/cedricziel/assistant/commit/3972b172ef323a52e8bcd8a55e3feb247837a2ca))

## [0.1.60](https://github.com/cedricziel/assistant/compare/v0.1.59...v0.1.60) (2026-03-27)


### Bug Fixes

* **slack:** persist active threads to SQLite so bot responds after restart ([#316](https://github.com/cedricziel/assistant/issues/316)) ([99ba80c](https://github.com/cedricziel/assistant/commit/99ba80cdc7ec80b94a785eb789b12475697a4e9d))

## [0.1.59](https://github.com/cedricziel/assistant/compare/v0.1.58...v0.1.59) (2026-03-26)


### Bug Fixes

* **runtime:** address compaction correctness issues from PR [#313](https://github.com/cedricziel/assistant/issues/313) review ([#314](https://github.com/cedricziel/assistant/issues/314)) ([eacf1c7](https://github.com/cedricziel/assistant/commit/eacf1c710188f90c97c74c1c40782b17a5655d04))

## [0.1.58](https://github.com/cedricziel/assistant/compare/v0.1.57...v0.1.58) (2026-03-26)


### Features

* **runtime:** add context compaction with token-threshold history summarization ([#312](https://github.com/cedricziel/assistant/issues/312)) ([312790f](https://github.com/cedricziel/assistant/commit/312790f3afee13239313b7456590671842ecdbb1))
* **runtime:** add context compaction with token-threshold history summarization ([#313](https://github.com/cedricziel/assistant/issues/313)) ([8dfde7e](https://github.com/cedricziel/assistant/commit/8dfde7eb4592edc18eba9c8aeb76f55c6ed35092))
* **runtime:** index conversation history into memory_chunks after each turn ([#311](https://github.com/cedricziel/assistant/issues/311)) ([ee69a1d](https://github.com/cedricziel/assistant/commit/ee69a1da4c31161a3e698d45a103a76e01a2fcbd))
* **tool-executor:** improve memory-search with query expansion, temporal decay, MMR, and multilingual stop words ([#309](https://github.com/cedricziel/assistant/issues/309)) ([57ceb80](https://github.com/cedricziel/assistant/commit/57ceb80132b7bfbfadb479b0906d7b3f709963bb))

## [0.1.57](https://github.com/cedricziel/assistant/compare/v0.1.56...v0.1.57) (2026-03-26)


### Bug Fixes

* **cli:** spawn interface workers in orchestrator run mode ([#307](https://github.com/cedricziel/assistant/issues/307)) ([1fe6301](https://github.com/cedricziel/assistant/commit/1fe6301ffd2a176a8967dc4f9038debdace08ffd))

## [0.1.56](https://github.com/cedricziel/assistant/compare/v0.1.55...v0.1.56) (2026-03-25)


### Features

* **runtime:** split persona-bound and anonymous subagent execution scope ([#300](https://github.com/cedricziel/assistant/issues/300)) ([8edf21f](https://github.com/cedricziel/assistant/commit/8edf21fa7c4205649d74fb0cb319c77ff4966a3c))


### Bug Fixes

* **runtime:** attribute subagent telemetry to parent persona ([#299](https://github.com/cedricziel/assistant/issues/299)) ([fd7ce44](https://github.com/cedricziel/assistant/commit/fd7ce44c8f5f0a2101121f81394d782cd314539e))
* **runtime:** persist subagent parent lineage from caller context ([#298](https://github.com/cedricziel/assistant/issues/298)) ([ad1ecf5](https://github.com/cedricziel/assistant/commit/ad1ecf57b2a8eb0fb9d259a5e551a7f803b46499))
* **runtime:** scope thinking-step history load to anonymous subagent persona ([#306](https://github.com/cedricziel/assistant/issues/306)) ([9fc036e](https://github.com/cedricziel/assistant/commit/9fc036e385c70975b2edbd2f269a5e47cfb23d8c))
* **storage:** decouple telemetry traces from conversations ([#281](https://github.com/cedricziel/assistant/issues/281)) ([c5b7e14](https://github.com/cedricziel/assistant/commit/c5b7e1418249dbdeb3f6ba71975b51559f32e820))

## [0.1.55](https://github.com/cedricziel/assistant/compare/v0.1.54...v0.1.55) (2026-03-24)


### Bug Fixes

* **release:** trigger release-please ([23725b9](https://github.com/cedricziel/assistant/commit/23725b9928ac188a7027b470be8eae85bdf4cf76))

## [0.1.54](https://github.com/cedricziel/assistant/compare/v0.1.53...v0.1.54) (2026-03-24)


### Bug Fixes

* **release:** install modern protoc in cross aarch64 builds ([#276](https://github.com/cedricziel/assistant/issues/276)) ([fa4ac34](https://github.com/cedricziel/assistant/commit/fa4ac346357370c13c4bdb2360bf5ea4986b7a87))

## [0.1.53](https://github.com/cedricziel/assistant/compare/v0.1.52...v0.1.53) (2026-03-24)


### Features

* **interface-cli:** unify runtime entrypoints and service migration ([#264](https://github.com/cedricziel/assistant/issues/264)) ([f9fe94c](https://github.com/cedricziel/assistant/commit/f9fe94c347f46952f46e93be2ef81be39ed34dbb))

## [0.1.52](https://github.com/cedricziel/assistant/compare/v0.1.51...v0.1.52) (2026-03-24)


### Bug Fixes

* **interface-cli:** prevent duplicate turn consumption in interface modes ([#262](https://github.com/cedricziel/assistant/issues/262)) ([6b2f9f6](https://github.com/cedricziel/assistant/commit/6b2f9f61624ed5effaef900d7c3525cfcb350383))

## [0.1.51](https://github.com/cedricziel/assistant/compare/v0.1.50...v0.1.51) (2026-03-24)


### Features

* **observability:** add submit_turn lifecycle and correlation telemetry ([#259](https://github.com/cedricziel/assistant/issues/259)) ([ab0a005](https://github.com/cedricziel/assistant/commit/ab0a00580b63618be6910e41819207f31bfde3e4))
* **web-ui:** redesign workflow detail and editor navigation ([#260](https://github.com/cedricziel/assistant/issues/260)) ([7825d59](https://github.com/cedricziel/assistant/commit/7825d59ac68d9190ad7f64bfeeb88607c9bde169))

## [0.1.50](https://github.com/cedricziel/assistant/compare/v0.1.49...v0.1.50) (2026-03-23)


### Features

* **web-ui:** make workflow editor mobile-first and Stimulus-driven ([#256](https://github.com/cedricziel/assistant/issues/256)) ([e23450a](https://github.com/cedricziel/assistant/commit/e23450ab6251eb91cc597808cdc1d6af9b47a373))


### Bug Fixes

* **binaries:** add clap version flags to all clap programs ([#254](https://github.com/cedricziel/assistant/issues/254)) ([924aab7](https://github.com/cedricziel/assistant/commit/924aab75ac3698f8f9c6ceebf888ee2ef3e752b3))
* **interface-slack:** align websocket TLS dependency versions ([#257](https://github.com/cedricziel/assistant/issues/257)) ([0e88526](https://github.com/cedricziel/assistant/commit/0e8852642fe083f43791dbd272c76d766f5ddd9f))

## [0.1.49](https://github.com/cedricziel/assistant/compare/v0.1.48...v0.1.49) (2026-03-23)


### Features

* **web-ui:** introduce Stimulus for workflow secret reveal ([#252](https://github.com/cedricziel/assistant/issues/252)) ([754de60](https://github.com/cedricziel/assistant/commit/754de6077297dae0f08304c44c0ade1c8ce00a6c))


### Bug Fixes

* **observability:** propagate root spans across bus and webhook flows ([#250](https://github.com/cedricziel/assistant/issues/250)) ([fff345d](https://github.com/cedricziel/assistant/commit/fff345d9012db93c8359c957c4db9c23f001a665))
* **web-ui:** harden workflow detail UX and mobile navigation ([#253](https://github.com/cedricziel/assistant/issues/253)) ([6b1cf26](https://github.com/cedricziel/assistant/commit/6b1cf2688b657c2a566bc0dc6267575a27f26064))

## [0.1.48](https://github.com/cedricziel/assistant/compare/v0.1.47...v0.1.48) (2026-03-23)


### Bug Fixes

* **observability:** separate service and span names across telemetry ([#247](https://github.com/cedricziel/assistant/issues/247)) ([cde3b18](https://github.com/cedricziel/assistant/commit/cde3b18d6deeafa7d13bbba9abed70821d77af59))
* **web-ui:** follow up PR247 review feedback ([#249](https://github.com/cedricziel/assistant/issues/249)) ([f170099](https://github.com/cedricziel/assistant/commit/f17009974eded36c21b45734ebee34e26ed589d8))
* **web-ui:** restore scrolling on workflow pages ([#245](https://github.com/cedricziel/assistant/issues/245)) ([eb657f4](https://github.com/cedricziel/assistant/commit/eb657f4ccd55f5ba971bb2cd08b701dac58371b4))

## [0.1.47](https://github.com/cedricziel/assistant/compare/v0.1.46...v0.1.47) (2026-03-23)


### Bug Fixes

* **ci:** run smoke tests against GitHub Actions Ollama service ([#243](https://github.com/cedricziel/assistant/issues/243)) ([60e4abd](https://github.com/cedricziel/assistant/commit/60e4abd073187c3c37efbb1351a1a11651d03b9c))
* **web-ui:** harden responsive navigation and accessibility ([#241](https://github.com/cedricziel/assistant/issues/241)) ([e79bd5b](https://github.com/cedricziel/assistant/commit/e79bd5b11661b09041e5f2603705095e62af5a1e))

## [0.1.46](https://github.com/cedricziel/assistant/compare/v0.1.45...v0.1.46) (2026-03-22)


### Features

* **web-ui:** redesign workflow form for desktop split layout ([#235](https://github.com/cedricziel/assistant/issues/235)) ([b2d1c21](https://github.com/cedricziel/assistant/commit/b2d1c214f2c6846a5c62f670d9b021455bf24591))


### Bug Fixes

* **release:** install protoc in aarch64 cross pre-build ([#232](https://github.com/cedricziel/assistant/issues/232)) ([1596a47](https://github.com/cedricziel/assistant/commit/1596a4767f5053f08880ae92e3d6f262b12c7ed6))
* **storage:** make agent-scope migrations sqlite-safe ([#233](https://github.com/cedricziel/assistant/issues/233)) ([2a7b5f4](https://github.com/cedricziel/assistant/commit/2a7b5f4d43244c27336192f87a21786e9866d1c1))

## [0.1.45](https://github.com/cedricziel/assistant/compare/v0.1.44...v0.1.45) (2026-03-22)


### Bug Fixes

* **cli:** enable clap env attribute support ([0949a4e](https://github.com/cedricziel/assistant/commit/0949a4e01fa761e2d187fc9c87ff1874fdbefdd7))

## [0.1.44](https://github.com/cedricziel/assistant/compare/v0.1.43...v0.1.44) (2026-03-22)


### Features

* **workflows:** add loop-enabled workflow graph management ([#228](https://github.com/cedricziel/assistant/issues/228)) ([fe883e5](https://github.com/cedricziel/assistant/commit/fe883e517e096c46701a4c60077bbfee563ce195))

## [0.1.43](https://github.com/cedricziel/assistant/compare/v0.1.42...v0.1.43) (2026-03-22)


### Features

* **runtime:** introduce multi-agent contexts with scoped runtime and web UI ([#227](https://github.com/cedricziel/assistant/issues/227)) ([f3226b1](https://github.com/cedricziel/assistant/commit/f3226b14aa8aa0b5d189f8582499811928d6da8a))
* **web-ui:** add health and readiness endpoints ([#225](https://github.com/cedricziel/assistant/issues/225)) ([786ac5c](https://github.com/cedricziel/assistant/commit/786ac5c33273321af4ee0ca2020eb12e4dc395d7))

## [0.1.42](https://github.com/cedricziel/assistant/compare/v0.1.41...v0.1.42) (2026-03-20)


### Features

* add NATS JetStream message bus backend ([#223](https://github.com/cedricziel/assistant/issues/223)) ([9cd2c31](https://github.com/cedricziel/assistant/commit/9cd2c317e720c6d30a5e948233e392bed4e7b61c))

## [0.1.41](https://github.com/cedricziel/assistant/compare/v0.1.40...v0.1.41) (2026-03-16)


### Miscellaneous Chores

* release 0.1.41 ([e0853e1](https://github.com/cedricziel/assistant/commit/e0853e114970732ffe918b7c1ff67fc0ec16622b))

## [0.1.40](https://github.com/cedricziel/assistant/compare/v0.1.39...v0.1.40) (2026-03-13)


### Features

* integrate external MCP servers as tool sources ([#195](https://github.com/cedricziel/assistant/issues/195)) ([8eab50e](https://github.com/cedricziel/assistant/commit/8eab50e1acc339994646ce2592cd6dbd0e2eb2a4))
* **interface-nextcloud:** add Nextcloud Talk webhook-based bot interface ([#196](https://github.com/cedricziel/assistant/issues/196)) ([ec98096](https://github.com/cedricziel/assistant/commit/ec9809672937e7b9eafe110f0ad59b21625b7e1d))


### Bug Fixes

* **cli:** spawn turn worker in standalone interface modes ([#204](https://github.com/cedricziel/assistant/issues/204)) ([d2104d3](https://github.com/cedricziel/assistant/commit/d2104d35f5a06dcf699ebb78d5ed3daea0f8904c))
* **llm:** make retry config injectable and lower default from 20 to 3 ([#207](https://github.com/cedricziel/assistant/issues/207)) ([4dca418](https://github.com/cedricziel/assistant/commit/4dca418f3398577797d561a3a59a6a053a3b6cab))
* **mcp-client:** address CodeRabbit review round 2 ([#205](https://github.com/cedricziel/assistant/issues/205)) ([a036bf0](https://github.com/cedricziel/assistant/commit/a036bf0a49e7f40750664ee4685b71201054fb28))
* patch nextcloud version ([#208](https://github.com/cedricziel/assistant/issues/208)) ([daa8dd9](https://github.com/cedricziel/assistant/commit/daa8dd97368ccfef12f0fb0e8f076f8c4edbe6ae))
* **slack:** only respond in threads where the bot was @-mentioned ([#206](https://github.com/cedricziel/assistant/issues/206)) ([7b174e9](https://github.com/cedricziel/assistant/commit/7b174e92f3da6ce9e011ed22885fcc8793aab52b))

## [0.1.39](https://github.com/cedricziel/assistant/compare/v0.1.38...v0.1.39) (2026-03-05)


### Features

* add current timestamp to system prompt ([#193](https://github.com/cedricziel/assistant/issues/193)) ([e9c9db8](https://github.com/cedricziel/assistant/commit/e9c9db8a4a6c92ef86255573de72c26e8c94d329))

## [0.1.38](https://github.com/cedricziel/assistant/compare/v0.1.37...v0.1.38) (2026-03-05)


### Features

* **llm:** increase default retry to 20 attempts with 60s max delay ([de7c625](https://github.com/cedricziel/assistant/commit/de7c625114a99c33be7f20ea1e20d72d134471fc))

## [0.1.37](https://github.com/cedricziel/assistant/compare/v0.1.36...v0.1.37) (2026-03-05)


### Bug Fixes

* **ci:** include both amd64 and arm64 debs in APT Packages index ([d375a1b](https://github.com/cedricziel/assistant/commit/d375a1b2431981a396af050bc6a32293a285a963))

## [0.1.36](https://github.com/cedricziel/assistant/compare/v0.1.35...v0.1.36) (2026-03-05)


### Features

* **runtime:** re-implement memory indexer ([#188](https://github.com/cedricziel/assistant/issues/188)) ([97d9016](https://github.com/cedricziel/assistant/commit/97d90163ba1c90424aa35c487148000c0f4da8b8))

## [0.1.35](https://github.com/cedricziel/assistant/compare/v0.1.34...v0.1.35) (2026-03-04)


### Features

* **llm:** add retry with exponential backoff for transient API errors ([#186](https://github.com/cedricziel/assistant/issues/186)) ([088a0c1](https://github.com/cedricziel/assistant/commit/088a0c17f7c31dd6e6f220d00700a21f6385693b)), closes [#183](https://github.com/cedricziel/assistant/issues/183)


### Bug Fixes

* **otel-exporter-sqlite:** provide Tokio context for batch processor threads ([#184](https://github.com/cedricziel/assistant/issues/184)) ([f635b3c](https://github.com/cedricziel/assistant/commit/f635b3c082f3afad7fbb9e1a6e2539b0f22923bd))

## [0.1.34](https://github.com/cedricziel/assistant/compare/v0.1.33...v0.1.34) (2026-03-02)


### Bug Fixes

* **ci:** use force-with-lease for gh-pages deploy safety ([#181](https://github.com/cedricziel/assistant/issues/181)) ([26f63cf](https://github.com/cedricziel/assistant/commit/26f63cfe0fbefcc925676cdb0c2f2bbe872a3b06))

## [0.1.33](https://github.com/cedricziel/assistant/compare/v0.1.32...v0.1.33) (2026-03-02)


### Bug Fixes

* **transcription:** log file magic bytes and preserve input on failure ([#178](https://github.com/cedricziel/assistant/issues/178)) ([8258766](https://github.com/cedricziel/assistant/commit/825876694f0d5d8588a4248d227a7c6decb7e776))

## [0.1.32](https://github.com/cedricziel/assistant/compare/v0.1.31...v0.1.32) (2026-03-02)


### Bug Fixes

* **transcription:** improve error logging for audio conversion failures ([#176](https://github.com/cedricziel/assistant/issues/176)) ([23b34c6](https://github.com/cedricziel/assistant/commit/23b34c601c768614abd404299be4144a3f92d143))

## [0.1.31](https://github.com/cedricziel/assistant/compare/v0.1.30...v0.1.31) (2026-03-02)


### Bug Fixes

* **slack:** set presence on every reconnect so bot appears online ([#172](https://github.com/cedricziel/assistant/issues/172)) ([21fc06d](https://github.com/cedricziel/assistant/commit/21fc06d1e1aa27fb8f35d4bad6cf7b068a73ec3d))
* **transcription:** use atomic nonce for unique temp file names ([#175](https://github.com/cedricziel/assistant/issues/175)) ([4e21274](https://github.com/cedricziel/assistant/commit/4e21274b2904b8b6827a6e49e099e61f37c1a60f))

## [0.1.30](https://github.com/cedricziel/assistant/compare/v0.1.29...v0.1.30) (2026-03-02)


### Features

* **runtime:** extract shared helpers and decompose ReAct loops ([#164](https://github.com/cedricziel/assistant/issues/164)) ([080b427](https://github.com/cedricziel/assistant/commit/080b427c304b5a9ffa87fc1f5be7e2508ac55db6))


### Bug Fixes

* **runtime:** persist subagent thinking steps to DB ([#167](https://github.com/cedricziel/assistant/issues/167)) ([92e1ab0](https://github.com/cedricziel/assistant/commit/92e1ab0d51ab862ee61ebdae100c989489545f74))
* **runtime:** record max_iterations error in metrics ([#169](https://github.com/cedricziel/assistant/issues/169)) ([f8cdd46](https://github.com/cedricziel/assistant/commit/f8cdd4677baefa2bfe5b8ea5178facb065ce1ff5))
* **runtime:** use tokio::fs for BOOT.md and HEARTBEAT.md reads ([#170](https://github.com/cedricziel/assistant/issues/170)) ([9b39de5](https://github.com/cedricziel/assistant/commit/9b39de5501eff5be94e86a406c23e7cabc438235))

## [0.1.29](https://github.com/cedricziel/assistant/compare/v0.1.28...v0.1.29) (2026-03-02)


### Features

* **deps:** upgrade opentelemetry 0.24 to 0.31 and related crates ([#145](https://github.com/cedricziel/assistant/issues/145)) ([c08f192](https://github.com/cedricziel/assistant/commit/c08f1926be0b8c18a0e8e47b660ad78a33efadfd))
* **skills:** sync embedded builtin skills to disk on startup ([#140](https://github.com/cedricziel/assistant/issues/140)) ([ca5c9cf](https://github.com/cedricziel/assistant/commit/ca5c9cfd7a997e8ad831e396154278ea6bcc5b5c)), closes [#81](https://github.com/cedricziel/assistant/issues/81)


### Bug Fixes

* **storage:** set global PRAGMA busy_timeout to avoid SQLITE_BUSY ([#152](https://github.com/cedricziel/assistant/issues/152)) ([09e9cb3](https://github.com/cedricziel/assistant/commit/09e9cb315220b3b708460b392a062ebc1b02b6c8))


### Performance Improvements

* **storage:** wrap OTel SQLite exporters in batch transactions ([#151](https://github.com/cedricziel/assistant/issues/151)) ([8ec7a28](https://github.com/cedricziel/assistant/commit/8ec7a283ceac3d3211eee593d0577bf67bd2ef87))

## [0.1.28](https://github.com/cedricziel/assistant/compare/v0.1.27...v0.1.28) (2026-03-01)


### Features

* **skills:** expose compatibility field in list-skills and declare requirements ([#138](https://github.com/cedricziel/assistant/issues/138)) ([f1b62ff](https://github.com/cedricziel/assistant/commit/f1b62ff9aa696ce8c6ff51b151966d1bd9758679))
* **transcription:** Add audio format conversion for Deepgram ([#139](https://github.com/cedricziel/assistant/issues/139)) ([b151736](https://github.com/cedricziel/assistant/commit/b1517367029c161d3d66da9182bcd19855dbf661))


### Bug Fixes

* **runtime:** replace tracing .enter() guards with .instrument() in async code ([#132](https://github.com/cedricziel/assistant/issues/132)) ([21206a2](https://github.com/cedricziel/assistant/commit/21206a2164c7462c58042aefd66a00ad4444742b)), closes [#116](https://github.com/cedricziel/assistant/issues/116)

## [0.1.27](https://github.com/cedricziel/assistant/compare/v0.1.26...v0.1.27) (2026-03-01)


### Features

* add voice message transcription (Whisper, Ollama, Deepgram) ([#131](https://github.com/cedricziel/assistant/issues/131)) ([9610c0d](https://github.com/cedricziel/assistant/commit/9610c0d7fe0f335875d9576a346aeee9f65b6ebb))
* **provider-openai,provider-moonshot:** add hosted web search support ([#126](https://github.com/cedricziel/assistant/issues/126)) ([a90de0a](https://github.com/cedricziel/assistant/commit/a90de0a26fe8af1cdcf210b83e365b5b1a6490f2))
* **provider-openai:** migrate from Chat Completions to Responses API ([#128](https://github.com/cedricziel/assistant/issues/128)) ([89eeffb](https://github.com/cedricziel/assistant/commit/89eeffbe4772565d83f28a8ec7786e89ff10a59f))


### Bug Fixes

* **runtime:** scope turn workers by interface to prevent cross-service theft ([#130](https://github.com/cedricziel/assistant/issues/130)) ([9b672b2](https://github.com/cedricziel/assistant/commit/9b672b2b6d405dee7264fe49dd293c6d91acdfeb))

## [0.1.26](https://github.com/cedricziel/assistant/compare/v0.1.25...v0.1.26) (2026-03-01)


### Features

* **provider-moonshot:** add Moonshot AI (Kimi) LLM provider ([#124](https://github.com/cedricziel/assistant/issues/124)) ([0c438ed](https://github.com/cedricziel/assistant/commit/0c438ed4b84ece11fe1114f82f871f74500ff718))


### Bug Fixes

* **storage:** cast metric aggregations to REAL for sqlx f64 decoding ([3194520](https://github.com/cedricziel/assistant/commit/31945205b355a0c0d78f6ca0df95ab51f472d789))

## [0.1.25](https://github.com/cedricziel/assistant/compare/v0.1.24...v0.1.25) (2026-03-01)


### Bug Fixes

* **ci:** add make vendor to release build workflow ([38aba3d](https://github.com/cedricziel/assistant/commit/38aba3d1aa08659c2e68e31a41d606922012d3ec))

## [0.1.24](https://github.com/cedricziel/assistant/compare/v0.1.23...v0.1.24) (2026-03-01)


### Features

* **web-ui:** route chat through Orchestrator for full assistant capabilities ([#121](https://github.com/cedricziel/assistant/issues/121)) ([7aabc85](https://github.com/cedricziel/assistant/commit/7aabc850b84ca38574fd96d1bcd59a7a3198d656))


### Bug Fixes

* **ci:** never cancel in-progress CI runs on main ([ac76ebf](https://github.com/cedricziel/assistant/commit/ac76ebfe6f4bfcefff49e58774ad22f23aa39e6d))
* **web-ui:** increase screenshot diff tolerance to 5% for cross-platform fonts ([969bf4a](https://github.com/cedricziel/assistant/commit/969bf4a122b498a24dfc75127034ac2b9a759a82))

## [0.1.23](https://github.com/cedricziel/assistant/compare/v0.1.22...v0.1.23) (2026-02-28)


### Bug Fixes

* **upload:** replace content_base64 with path param to prevent binary data in LLM context ([#118](https://github.com/cedricziel/assistant/issues/118)) ([84f08c6](https://github.com/cedricziel/assistant/commit/84f08c6b87786126514a5eb5beea7ccaac2427a0))

## [0.1.22](https://github.com/cedricziel/assistant/compare/v0.1.21...v0.1.22) (2026-02-28)


### Features

* **llm:** add dedicated embedding provider with Voyage AI support ([#109](https://github.com/cedricziel/assistant/issues/109)) ([b71ff13](https://github.com/cedricziel/assistant/commit/b71ff137b8d929159b3cd7af23e2e5dd19d9a9c7))
* **provider-openai:** add OpenAI LLM provider with API key and OAuth PKCE auth ([#105](https://github.com/cedricziel/assistant/issues/105)) ([af44f8a](https://github.com/cedricziel/assistant/commit/af44f8a7609c9bc9cbdec3042cc9ca97f633ba3c))
* **web-ui:** add PWA support for installable offline-capable app ([#112](https://github.com/cedricziel/assistant/issues/112)) ([395db47](https://github.com/cedricziel/assistant/commit/395db47b4647ed27c5ed63fc5809dfcb90ef1efb))
* **web-ui:** chat interface with LLM streaming and Askama template migration ([#107](https://github.com/cedricziel/assistant/issues/107)) ([e0de5b9](https://github.com/cedricziel/assistant/commit/e0de5b94866b399577dabd12a49438e0a192c399))


### Bug Fixes

* **interface-slack:** strip cite tags and convert markdown tables in mrkdwn output ([#108](https://github.com/cedricziel/assistant/issues/108)) ([b88a93b](https://github.com/cedricziel/assistant/commit/b88a93b2596073791d16e1494d8349719cdd04a6))
* truncate bash output and increase turn timeout ([#110](https://github.com/cedricziel/assistant/issues/110)) ([67a1327](https://github.com/cedricziel/assistant/commit/67a132729b971f070d2cac62c082001da10539e3))

## [0.1.21](https://github.com/cedricziel/assistant/compare/v0.1.20...v0.1.21) (2026-02-28)


### Features

* **web-ui:** add --no-secure-cookie flag for plain HTTP on non-loopback ([#103](https://github.com/cedricziel/assistant/issues/103)) ([901db66](https://github.com/cedricziel/assistant/commit/901db660b79d7baa46972402639b5996c0b4b622))

## [0.1.20](https://github.com/cedricziel/assistant/compare/v0.1.19...v0.1.20) (2026-02-28)


### Bug Fixes

* **ci:** build and ship assistant-web-ui binary in release ([#101](https://github.com/cedricziel/assistant/issues/101)) ([06c55cc](https://github.com/cedricziel/assistant/commit/06c55cc9eeca9e860b4275900671fc54903e3889))

## [0.1.19](https://github.com/cedricziel/assistant/compare/v0.1.18...v0.1.19) (2026-02-28)


### Bug Fixes

* **packaging:** correct web-ui binary name and add EnvironmentFile ([#99](https://github.com/cedricziel/assistant/issues/99)) ([5848347](https://github.com/cedricziel/assistant/commit/58483474330008b66d8aade75abf1ad43ec13bdb))

## [0.1.18](https://github.com/cedricziel/assistant/compare/v0.1.17...v0.1.18) (2026-02-27)


### Features

* **web-ui:** add token-based authentication ([#98](https://github.com/cedricziel/assistant/issues/98)) ([bf479d4](https://github.com/cedricziel/assistant/commit/bf479d4fcd34c25f4059455a4c26018454ebe718))
* **web-ui:** add webhook management with HMAC-SHA256 verification ([#95](https://github.com/cedricziel/assistant/issues/95)) ([0d3af9b](https://github.com/cedricziel/assistant/commit/0d3af9b2fb27e9ecaa8c457d86953d3b6bbd413c))

## [0.1.17](https://github.com/cedricziel/assistant/compare/v0.1.16...v0.1.17) (2026-02-27)


### Features

* **skill:** add coding-agent skill for multi-agent background orchestration ([#92](https://github.com/cedricziel/assistant/issues/92)) ([60d73c8](https://github.com/cedricziel/assistant/commit/60d73c847f53b2e766f5909dbaf0c53fedfd08a7))
* **skills:** add playwright-cli skill ([#70](https://github.com/cedricziel/assistant/issues/70)) ([4c48d9c](https://github.com/cedricziel/assistant/commit/4c48d9c947130b2269b32737d72fe493038be212))
* **slack:** add listen mode to control which messages the bot reacts to ([#93](https://github.com/cedricziel/assistant/issues/93)) ([3cea363](https://github.com/cedricziel/assistant/commit/3cea363cd85972dbd3e677b5fc685d18a4de8b0a))
* **tool-executor:** add native process tool for background process management ([#88](https://github.com/cedricziel/assistant/issues/88)) ([0ca39c6](https://github.com/cedricziel/assistant/commit/0ca39c62df3387b01baa8ca974ea532b37907167))


### Bug Fixes

* **skill:** use stdin for prompt delivery to avoid shell escaping issues ([#86](https://github.com/cedricziel/assistant/issues/86)) ([c3757dd](https://github.com/cedricziel/assistant/commit/c3757dd8ae9fb7d318ad1a806af72a243b1b327b))

## [0.1.16](https://github.com/cedricziel/assistant/compare/v0.1.15...v0.1.16) (2026-02-27)


### Features

* add A2A protocol support with agent management UI ([#69](https://github.com/cedricziel/assistant/issues/69)) ([58c68e5](https://github.com/cedricziel/assistant/commit/58c68e556f263ee9359f890959d4d27da2abf8b3))
* add OpenTelemetry metrics with SQLite persistence and analytics dashboard ([#76](https://github.com/cedricziel/assistant/issues/76)) ([6d1364c](https://github.com/cedricziel/assistant/commit/6d1364c5bbd206ff73c3da273b0ece8362d77b3a))
* **skill:** add claude-code-agent skill with async tmux support ([#75](https://github.com/cedricziel/assistant/issues/75)) ([4a052a1](https://github.com/cedricziel/assistant/commit/4a052a147b68a572c62581a5fdb466039703b9f3))


### Bug Fixes

* sync Cargo.lock versions for a2a crates after release bump ([d0dd77e](https://github.com/cedricziel/assistant/commit/d0dd77e50ac60451503944655022643c7cfffaf8))
* use literal version strings in a2a crates for release-please compatibility ([#84](https://github.com/cedricziel/assistant/issues/84)) ([7231c50](https://github.com/cedricziel/assistant/commit/7231c5022081761ba5d555f7aee3d3248f5c4331))

## [0.1.15](https://github.com/cedricziel/assistant/compare/v0.1.14...v0.1.15) (2026-02-26)


### Features

* add subagent support with tool filtering, lifecycle tracking, and OTel observability ([#67](https://github.com/cedricziel/assistant/issues/67)) ([dba4255](https://github.com/cedricziel/assistant/commit/dba4255c7d57638fa30714bff8d99f01f3058e4c))

## [0.1.14](https://github.com/cedricziel/assistant/compare/v0.1.13...v0.1.14) (2026-02-26)


### Bug Fixes

* **cli:** spawn scheduler before interface branches so all modes get scheduled tasks ([#65](https://github.com/cedricziel/assistant/issues/65)) ([be929fb](https://github.com/cedricziel/assistant/commit/be929fb62cdf821d8df4c57e668092d6d27b2c8a))

## [0.1.13](https://github.com/cedricziel/assistant/compare/v0.1.12...v0.1.13) (2026-02-26)


### Features

* add durable topic-based message bus for inter-component communication ([#63](https://github.com/cedricziel/assistant/issues/63)) ([dd6a520](https://github.com/cedricziel/assistant/commit/dd6a52004206ab08843817089e4326f1a55af772))

## [0.1.12](https://github.com/cedricziel/assistant/compare/v0.1.11...v0.1.12) (2026-02-26)


### Features

* **core:** extract memory templates to .md files, add TOOLS/BOOTSTRAP/HEARTBEAT/BOOT ([#60](https://github.com/cedricziel/assistant/issues/60)) ([0837a23](https://github.com/cedricziel/assistant/commit/0837a23515dc60842ca7503a1e1d47c70c24c2d1))


### Bug Fixes

* **packaging:** use /run/systemd/users instead of loginctl for service restart ([#58](https://github.com/cedricziel/assistant/issues/58)) ([d4ff032](https://github.com/cedricziel/assistant/commit/d4ff032460acc6eb04b9ff72d9878a6dc9dc2db3))
* **slack,mattermost:** handle whitespace, no-pad, and data-URI in base64 uploads ([#62](https://github.com/cedricziel/assistant/issues/62)) ([7f753ff](https://github.com/cedricziel/assistant/commit/7f753ff297ae8d9828d90c980a6173256884f680))

## [0.1.11](https://github.com/cedricziel/assistant/compare/v0.1.10...v0.1.11) (2026-02-26)


### Features

* **tool-executor:** add memory-append builtin tool ([#55](https://github.com/cedricziel/assistant/issues/55)) ([4bf16ed](https://github.com/cedricziel/assistant/commit/4bf16eda5f63f6b30086b3d50a5c357f5d99c312))

## [0.1.10](https://github.com/cedricziel/assistant/compare/v0.1.9...v0.1.10) (2026-02-26)


### Features

* add attachment/file sending support across all interfaces ([#56](https://github.com/cedricziel/assistant/issues/56)) ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **cli:** deliver file attachments from tool outputs to disk ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **core:** add Attachment type and attachment support to ToolOutput ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **mattermost:** add file upload tool with multipart support ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** collect attachments from tool outputs and add error recovery ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **slack:** add binary/base64 upload support to upload tool ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))


### Bug Fixes

* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([#53](https://github.com/cedricziel/assistant/issues/53)) ([e11cf60](https://github.com/cedricziel/assistant/commit/e11cf60dc3851f06ff44e9f4afd68d3f8950e082))

## [0.1.9](https://github.com/cedricziel/assistant/compare/v0.1.8...v0.1.9) (2026-02-25)


### Features

* **runtime:** add sysiphos.heartbeat root span to heartbeat traces ([#50](https://github.com/cedricziel/assistant/issues/50)) ([b468fb9](https://github.com/cedricziel/assistant/commit/b468fb9c90fff007f9784bf98baf105e0647e85a))


### Bug Fixes

* **packaging:** restart services on upgrade instead of leaving them dead ([#52](https://github.com/cedricziel/assistant/issues/52)) ([d61c1c1](https://github.com/cedricziel/assistant/commit/d61c1c171d255f601e0bea31b676cd20af0d18b4))

## [0.1.8](https://github.com/cedricziel/assistant/compare/v0.1.7...v0.1.8) (2026-02-25)


### Features

* **scheduler:** add cancel-task, list-tasks tools and one-shot scheduling ([#49](https://github.com/cedricziel/assistant/issues/49)) ([a9f84aa](https://github.com/cedricziel/assistant/commit/a9f84aa0ba878b705d6604ace771bf12b1118d8a))
* **slack:** receive file attachments with vision support ([#48](https://github.com/cedricziel/assistant/issues/48)) ([fc3fe5b](https://github.com/cedricziel/assistant/commit/fc3fe5bf8ed4fdb6ac2aa2b35d8cc3c3b28f825b))


### Bug Fixes

* **packaging:** use Restart=always so services recover after self-update ([#46](https://github.com/cedricziel/assistant/issues/46)) ([df4c964](https://github.com/cedricziel/assistant/commit/df4c9645937d807429f7b11d487865e574d1d541))

## [0.1.7](https://github.com/cedricziel/assistant/compare/v0.1.6...v0.1.7) (2026-02-25)


### Features

* add OTel log ingestion pipeline with web UI ([#44](https://github.com/cedricziel/assistant/issues/44)) ([0315aa3](https://github.com/cedricziel/assistant/commit/0315aa3d9496e1e9a5a03d00d19c5f91da3ad369))
* **interface-slack:** queue indicator and message stacking for Slack threads ([#42](https://github.com/cedricziel/assistant/issues/42)) ([8bd59a0](https://github.com/cedricziel/assistant/commit/8bd59a0d928070ecaac94ef09b7ece30f5e00331))


### Bug Fixes

* **runtime:** reject end_turn when LLM skips reply in messaging interfaces ([#45](https://github.com/cedricziel/assistant/issues/45)) ([c077b30](https://github.com/cedricziel/assistant/commit/c077b30dc296805b99870b1802691294ba2ba33d))

## [0.1.6](https://github.com/cedricziel/assistant/compare/v0.1.5...v0.1.6) (2026-02-25)


### Features

* add otel tracng ([9972d42](https://github.com/cedricziel/assistant/commit/9972d4205578370667a47fdfd4e0d361152c2027))
* **anthropic:** expose hosted web fetch tool ([40eb8ef](https://github.com/cedricziel/assistant/commit/40eb8ef8f4c8a673f25c1079093d70bef4b34581))
* **anthropic:** wire hosted web search tool ([e21bbf0](https://github.com/cedricziel/assistant/commit/e21bbf02afb0306dc73b2e14370b68ecbef3632e))
* **core:** add AGENTS.md — session startup ritual and memory discipline ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **interface-slack:** add 7 ambient Slack tools and fix thinking responses ([e533c83](https://github.com/cedricziel/assistant/commit/e533c830909683aab7904472ff2aeb9c0d329f66))
* **llm:** add provider metadata to LlmProvider trait ([552c42c](https://github.com/cedricziel/assistant/commit/552c42c58c4d4ad69038fe896f2f5f9d0bc43618))
* **llm:** add response metadata to LlmResponse ([c6f1550](https://github.com/cedricziel/assistant/commit/c6f15504559eae1d598375bc83bc612d551510b7))
* **observability:** add otel spans and trace UI ([141294b](https://github.com/cedricziel/assistant/commit/141294b10e032866c26e2f4055de2700c8160a78))
* **provider-anthropic:** add Anthropic Claude provider ([1d264c2](https://github.com/cedricziel/assistant/commit/1d264c2aed0a0fdbf8ba8613f006f9411ebde5af))
* redesign trace analytics ui ([c523298](https://github.com/cedricziel/assistant/commit/c52329889059b03f696e1a443d12a176e1227db8))
* **refactor:** separate Skills (knowledge) from Tools (executables) ([#36](https://github.com/cedricziel/assistant/issues/36)) ([fc81988](https://github.com/cedricziel/assistant/commit/fc81988d57f1f3a41a22f3d42fea67da72ec2cc8))
* **runtime:** add opt-in GenAI content capture on spans ([50eca82](https://github.com/cedricziel/assistant/commit/50eca8258c7d06d9422acfeb11b4d733fbfd8335))
* **runtime:** align spans with OTel GenAI semantic conventions ([77ddcdc](https://github.com/cedricziel/assistant/commit/77ddcdc66455fb268e2631c5561f1c071fc069a7))
* **runtime:** enrich self-analyze with token usage data ([27eea5a](https://github.com/cedricziel/assistant/commit/27eea5a8d3deae006e3f3a081fb3d1055f799b48))
* **runtime:** propagate OTel trace context across conversation turns ([b2bfaa4](https://github.com/cedricziel/assistant/commit/b2bfaa4c7c1975de789d75ae8bee785c66791ca1))
* **runtime:** raise default max_iterations to 80 ([d8ee35d](https://github.com/cedricziel/assistant/commit/d8ee35d5caefb4f1e478ecd8d665523f50f0a375))
* **signal:** propagate OTel trace context across conversation turns ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))
* **skills:** auto-discover external skill folders ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **skills:** enrich metadata parsing and surface available skills ([e5aa782](https://github.com/cedricziel/assistant/commit/e5aa7825f33191680569366f236893505539cb0c))
* **slack:** treat reactions as turns ([a4148b9](https://github.com/cedricziel/assistant/commit/a4148b98f5ec9c9583b9b530b887449dc9b7cc4c))
* **storage:** add token usage columns to distributed_traces ([a6e763a](https://github.com/cedricziel/assistant/commit/a6e763aba9bae6dcce7d2e89202639e8159ccbaf))
* **ui:** add web trace viewer ([eb41f9c](https://github.com/cedricziel/assistant/commit/eb41f9cbff151dca2ff769e42edbbe65db680b8e))
* **web-ui:** redesign trace analytics UI ([4857a9f](https://github.com/cedricziel/assistant/commit/4857a9f210769064df8b7f44be3fde62521d1e4a))


### Bug Fixes

* **ci:** use inline version strings for release-please compatibility ([#39](https://github.com/cedricziel/assistant/issues/39)) ([f3e3ed9](https://github.com/cedricziel/assistant/commit/f3e3ed95e2229b347679fb530d600e2988c50218))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([#37](https://github.com/cedricziel/assistant/issues/37)) ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **interface-slack:** use generic reply/react/upload extension tool names and hide slack-post during threaded turns ([b43164b](https://github.com/cedricziel/assistant/commit/b43164b60b722066a5b6434ea1e6bf41d1631b58))
* **llm:** handle empty content from thinking models (qwen3) ([46be85a](https://github.com/cedricziel/assistant/commit/46be85ab25e78e8d3a9fc82ccbe7251ae095e06e))
* **runtime:** record end_turn tool result ([5d10217](https://github.com/cedricziel/assistant/commit/5d10217f3ef55c4eb8b582895f4c79e075f8eb61))
* **signal:** add missing trace_cx parameter to run_turn_streaming ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))

## [0.1.5](https://github.com/cedricziel/assistant/compare/v0.1.4...v0.1.5) (2026-02-23)


### Features

* **ci:** publish APT/YUM package repo to GitHub Pages ([#34](https://github.com/cedricziel/assistant/issues/34)) ([b166763](https://github.com/cedricziel/assistant/commit/b16676325d6374bcd0479e24a7bea83d5349986d))

## [0.1.4](https://github.com/cedricziel/assistant/compare/v0.1.3...v0.1.4) (2026-02-23)


### Features

* **cli:** unified binary with ambient skill plugin architecture ([#32](https://github.com/cedricziel/assistant/issues/32)) ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))
* **packaging:** systemd user services for Slack and Mattermost bots ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))

## [0.1.3](https://github.com/cedricziel/assistant/compare/v0.1.2...v0.1.3) (2026-02-23)


### Features

* **memory:** memory-get + memory-search with FTS5/vector indexing; remove SafetyGate ([#27](https://github.com/cedricziel/assistant/issues/27)) ([9ddbace](https://github.com/cedricziel/assistant/commit/9ddbaceb2ed8207693dcfbb2175741513eb31129))

## [0.1.2](https://github.com/cedricziel/assistant/compare/v0.1.1...v0.1.2) (2026-02-22)


### Features

* add bash skill ([90d5330](https://github.com/cedricziel/assistant/commit/90d53306e11aae8f888cf3ea4dc80d9d54c9c889))
* **cli:** add reset subcommand to wipe all assistant data ([5670653](https://github.com/cedricziel/assistant/commit/5670653cdcd39674160fac092b973f59454b28f0))
* **core:** add LlmProviderKind to LlmConfig for future provider selection ([ed7abcb](https://github.com/cedricziel/assistant/commit/ed7abcb6e70b5e779a898f5693bf417b749961b9))
* **core:** embed bundled skills into the binary at compile time ([ae0969e](https://github.com/cedricziel/assistant/commit/ae0969e76e4dd081bc738da8518c4e383d3d182f))
* **core:** improve default memory file templates ([6c0c959](https://github.com/cedricziel/assistant/commit/6c0c959c24b919360fbebfd1d6990cc330f7f6dc))
* **core:** instruct LLM when and how to write daily notes ([e8f6cd1](https://github.com/cedricziel/assistant/commit/e8f6cd1ddc9b3ed85e805b560846d03fc9a1458a))
* **interface-slack:** presence and typing status indicators ([3b6ecf5](https://github.com/cedricziel/assistant/commit/3b6ecf5dfdf2cd49eb8b8ad20e5789da16026781))
* **llm:** add LlmProvider trait with Capabilities and ToolSupport ([920d3d3](https://github.com/cedricziel/assistant/commit/920d3d3efcff01f309add3b6f3007a7cd85172e6))
* **llm:** ChatHistoryMessage enum with structured tool-call variants ([96f17d7](https://github.com/cedricziel/assistant/commit/96f17d7c8812075ccaf07a105606dc28d5251ff3))
* **llm:** support multiple simultaneous tool calls ([7ec9d0d](https://github.com/cedricziel/assistant/commit/7ec9d0d43fff6b1f82d2792d01c8b1db88c0f53b))
* **provider-ollama:** new crate with OllamaProvider implementing LlmProvider ([ce6911d](https://github.com/cedricziel/assistant/commit/ce6911d9795d9884ce712163b32ed65461cf22b4))
* **runtime:** add end_turn tool and soften messaging-interface prompt ([8a9f832](https://github.com/cedricziel/assistant/commit/8a9f832f6c60798c1619e75e763ee8c01aa2cbb4))
* **scheduler:** wire scheduler and add heartbeat loop ([2043ee4](https://github.com/cedricziel/assistant/commit/2043ee43386fa4c2a15c3064016ffbfc303cc5d5))
* **skills-executor:** add file-read, file-write, file-edit, file-glob, web-search builtins ([a577eec](https://github.com/cedricziel/assistant/commit/a577eecd2514d76dd57ab8f761abed49dcd72f11))
* **skills-executor:** add memory-patch builtin skill ([1889fe3](https://github.com/cedricziel/assistant/commit/1889fe3f55e94ebf8368d6a7c4eb4ecbe4bc9e45))
* **storage:** persist tool-call and tool-result messages to DB ([e449b24](https://github.com/cedricziel/assistant/commit/e449b2459a832f7c3875668f2e8e2928db67acc2))
* **tools:** JSON Schema param validation and output_schema for ToolHandler ([25345f5](https://github.com/cedricziel/assistant/commit/25345f5319a5144835214cefffbd141772581b20))
* **tools:** proper JSON Schema for all ToolHandler param schemas ([7b383ec](https://github.com/cedricziel/assistant/commit/7b383ec8c0bac302d7ec9b037938ea8a1610e261))
* **tools:** wire output_schema and structured data into observations ([bf02232](https://github.com/cedricziel/assistant/commit/bf02232c2845f8b6c0e0c1c767f04f642ab21168))


### Bug Fixes

* **interface-slack,runtime:** prevent double replies and concurrent turns ([94d6bac](https://github.com/cedricziel/assistant/commit/94d6bac48b5ff35b673f478651a6baac4322fabc))
* **interface-slack:** convert Markdown to Slack mrkdwn before posting ([6e5767c](https://github.com/cedricziel/assistant/commit/6e5767c6c45ecc8c88602d36d8907bbef54c84bf))
* **runtime:** persist tool results for all early-exit paths in orchestrator ([5c96352](https://github.com/cedricziel/assistant/commit/5c9635251a1f9b57c083bb7ec5f4683172d2005f))
* **runtime:** prevent double-posting and wrong tool in Slack auto-post fallback ([1f4fc6b](https://github.com/cedricziel/assistant/commit/1f4fc6bb77e7bbec1b728b886631259920478071))
* **runtime:** require ack before end_turn in messaging interfaces ([9af5fbc](https://github.com/cedricziel/assistant/commit/9af5fbc1d02043312eb6d7d8321b1394c7b423c0))
* **skills:** correct memory-patch SKILL.md frontmatter format ([8cd1dee](https://github.com/cedricziel/assistant/commit/8cd1deedc3e6513c02d2fd2103a4ca5f2b481206))
* **storage:** make migration 005 idempotent with IF NOT EXISTS ([2deb103](https://github.com/cedricziel/assistant/commit/2deb1038184da2783584b87f831a3d4a2c300732))
* **storage:** revert IF NOT EXISTS — macOS system SQLite &lt; 3.37 unsupported ([5d8f730](https://github.com/cedricziel/assistant/commit/5d8f73035bc1c445b9dd41eb3e7cb2cd78b594d3))
* **storage:** track applied migrations to prevent re-running on each launch ([6d8d29b](https://github.com/cedricziel/assistant/commit/6d8d29b992573b63ad66232642fab3ea02e55d9c))
