# Nomad Timer

작은 Windows 네이티브 휴식 위젯입니다.

정시 기준으로 50분은 일하는 시간, 10분은 쉬는시간으로 맞추고, 모든 클라이언트는 같은 API 서버를 기준으로 같은 리듬을 봅니다.

## What It Does

- Windows 오른쪽 아래에 뜨는 작은 topmost 위젯
- Electron 없이 Rust + Win32 API로 구현
- 서버 기준 시각으로 전 세계 사용자의 휴식 타이밍 동기화
- 프리셋 반응 broadcast: 휴식, 기지개, 물, 인사, 응원, 복귀
- 상태와 최신 반응에 따라 바뀌는 픽셀 고양이
- 트레이 아이콘, 접기/펼치기, 닫기

## Public API

기본 API는 운영 서버를 사용합니다.

```text
https://nomad-timer.hyeon.space/api
```

Windows 앱은 `BREAKTIME_SERVER` 환경변수로 API 서버를 바꿀 수 있습니다.

```powershell
$env:BREAKTIME_SERVER="https://nomad-timer.hyeon.space/api"
.\target\release\breaktime.exe
```

API 서버 코드는 클라이언트와 분리해서 운영하는 것을 권장합니다. 이 저장소의 기본 빌드는 클라이언트와 사이트만 빌드하고, 서버 타깃은 `api-server` feature로 분리되어 있습니다.

## Build

Windows 앱:

```powershell
cargo build --release
```

API 서버 로컬 개발:

```powershell
cargo run --features api-server --bin breaktime-server
```

사이트:

```powershell
cd site
npm install
npm test
npm run build
```

## Repository Split

추천 구조:

- `nomad-timer`: Windows app, landing site, pixel cat assets, public API contract
- `nomad-timer-api`: hosted API server, rate limits, moderation, deployment config

클라이언트 repo는 API 서버를 몰라도 빌드되어야 하고, API repo는 클라이언트 릴리스 없이 독립 배포되어야 합니다.

## Contributing

이 프로젝트는 아직 작습니다. 좋은 변경 방향은 다음입니다.

- 위젯 드래그/위치 저장
- 작업표시줄 고정 모드
- 접근성 트리 개선
- 더 귀여운 픽셀 고양이
- 프리셋 반응 UX 개선
- API 서버 rate limit과 moderation 분리

PR은 작게 나눠 주세요. 앱 UI 변경은 가능하면 Windows에서 실제 캡처로 확인해 주세요.

## License

MIT
