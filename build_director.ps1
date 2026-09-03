# PalaceOS Director/Director Panel 을 release 로 빌드해서 production/director/
# 에 PalaceOS-Director.exe / PalaceOS-Director-Panel.exe 로 복사해준다.
#
# 그냥 `cargo build`(또는 `--release`)만 해서는 target/release/ 안에 원래
# 이름(director.exe, director_panel.exe)으로만 생기고 production/director/ 쪽은
# 전혀 안 바뀐다 — 이 스크립트가 그 복사+이름 바꾸기까지 한 번에 해준다.
# 실행: 프로젝트 루트에서 `powershell -ExecutionPolicy Bypass -File build_director.ps1`

$ErrorActionPreference = "Stop"

Write-Host "director / director_panel release 빌드 중..."
cargo build --release --bin director --bin director_panel
if ($LASTEXITCODE -ne 0) {
    Write-Host "빌드 실패 — 위 에러를 확인하세요." -ForegroundColor Red
    exit 1
}

$dst = "production\director"
New-Item -ItemType Directory -Force -Path $dst | Out-Null

Copy-Item "target\release\director.exe" "$dst\PalaceOS-Director.exe" -Force
Copy-Item "target\release\director_panel.exe" "$dst\PalaceOS-Director-Panel.exe" -Force

# 이전 실행에서 남은 상태 파일은 새로 시작할 때 헷갈리지 않게 지운다
# (녹화 이어붙이기/글리치 설정 등은 매번 새로 시작하는 게 낫다).
foreach ($f in @("director_state.json", "director_state.json.tmp", "director_status.json", "director_status.json.tmp", "director_shader_error.log", "audio_capture_error.log")) {
    Remove-Item "$dst\$f" -ErrorAction SilentlyContinue
}

Write-Host "완료: $dst\PalaceOS-Director.exe, $dst\PalaceOS-Director-Panel.exe" -ForegroundColor Green
