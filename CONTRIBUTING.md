# Contributing

고쳐 쓰기 쉬운 작은 앱을 목표로 합니다.

## Local Checks

```powershell
cargo test
cargo test --features api-server

cd site
npm test
npm run build
```

## Guidelines

- Electron, WebView, 무거운 UI 프레임워크는 피합니다.
- Windows 앱은 Rust + Win32 네이티브 방향을 유지합니다.
- API 클라이언트와 API 서버 구현은 분리합니다.
- 프리셋 broadcast는 준비된 메시지만 허용합니다.
- UI 변경은 텍스트 겹침, 작은 화면, 버튼 클릭 영역을 같이 확인합니다.
- 고양이 이미지는 앱 런타임용 BMP와 사이트용 PNG를 둘 다 관리합니다.

## API Changes

공개 API 계약을 바꿀 때는 `src/api_contract.rs`를 먼저 수정하고, 클라이언트 테스트와 서버 feature 테스트를 함께 갱신해 주세요.
