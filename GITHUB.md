# GitHub 관련 메모

저장소/계정/릴리스 방법을 잊지 않으려고 적어둔 문서. 게임 설정은 [README.md](README.md), 스토리는
[STORY.md](STORY.md), 제작 순서는 [ROADMAP.md](ROADMAP.md).

## 저장소

- 주소: https://github.com/shupil1021/Untitled (비공개)
- 계정: `shupil1021` — **옛 계정(`crackhead27515`)과 그 Gmail(`clanguagemaker@gmail.com`)은 잃어버려서
  못 쓴다.** 옛 저장소에는 아무것도 안 올라가 있다.
- 원격 이름은 `origin`, 브랜치는 `master`(처음부터 이 이름이라 `main` 이 아니다).
- 비공개라서 로그인한 본인만 코드/릴리스 파일을 받을 수 있다. 다른 사람에게 배포하려면 저장소를
  공개로 바꾸거나 파일을 직접 전달해야 한다.

## 커밋/푸시

```bash
git push            # 이미 origin/master 를 추적 중이라 이것만 하면 된다
```

- 이 컴퓨터에는 GitHub 로그인이 이미 되어 있다(Git Credential Manager + `gh`). 푸시가 로그인 창을
  띄우면 브라우저에서 `shupil1021` 로 로그인하면 된다. **비밀번호/토큰은 AI 에게 알려주지 않는다.**
- 커밋 작성자는 아직 `PalaceOS Dev <clanguagemaker@gmail.com>`(없어진 메일)이다. 새 GitHub 프로필과
  이어지게 하려면 `git config user.name/user.email` 을 새 계정 것으로 바꾸면 된다(앞으로의 커밋부터
  적용 — 이미 올라간 커밋까지 고치려면 기록을 다시 쓰고 강제 푸시해야 해서 따로 확인받고 한다).
- 지워진 에셋(`assets/photo/*` 등)의 삭제도 이미 커밋돼 있다. `.gitignore` 에 `target`, `production`,
  `*.pdb` 가 들어 있어 빌드 결과물과 녹화 도구 폴더는 올라가지 않는다.

## 릴리스 (진짜 Release — 태그만이 아니라)

현재 릴리스: **v0.1.0** (https://github.com/shupil1021/Untitled/releases/tag/v0.1.0), 파일 3개:

- `Untitled.exe` — 단독 실행 파일(에셋이 전부 exe 안에 들어 있다)
- `Untitled-v0.1.0-windows.zip` — exe + README
- `Untitled-v0.1.0-source.zip` — 소스 코드(`git archive`)

**자동 빌드(GitHub Actions)는 안 쓴다.** 태그를 올리면 자동으로 빌드하게 해봤는데 느리고(Test 단계가
오래 걸림) 릴리스가 안 보여서 지웠다. 릴리스는 이 컴퓨터에서 직접 빌드해서 `gh` 로 올린다.

### 새 버전 올리는 순서 (예: v0.1.1)

1. 코드를 커밋하고 `git push` 한다.
2. 릴리스 빌드 + 압축 (PowerShell):

```powershell
cargo build --release --bin crackhead
$d = "$env:TEMP\untitled_release"; New-Item -ItemType Directory $d -Force | Out-Null
Copy-Item target\release\crackhead.exe "$d\Untitled.exe"
Copy-Item README.md "$d\README.md"
Compress-Archive -Path "$d\Untitled.exe","$d\README.md" -DestinationPath "$d\Untitled-v0.1.1-windows.zip" -Force
git archive --format=zip --prefix="Untitled-v0.1.1/" -o "$d\Untitled-v0.1.1-source.zip" HEAD
```

3. 릴리스 생성 — `--target master` 라서 태그는 현재 `master` 커밋에 자동으로 만들어진다:

```powershell
gh release create v0.1.1 "$d\Untitled.exe" "$d\Untitled-v0.1.1-windows.zip" "$d\Untitled-v0.1.1-source.zip" `
  --repo shupil1021/Untitled --target master --title "Untitled v0.1.1" --notes "변경 내용"
```

### 다시 배포(재배포)할 때

릴리스만 지우면 **태그가 옛 커밋에 남아서** 새 릴리스가 옛 커밋을 가리킨다. 반드시 태그까지 같이
지우고 다시 만든다:

```powershell
gh release delete v0.1.0 --repo shupil1021/Untitled --cleanup-tag --yes
# 그 다음 위 3번처럼 gh release create (같은 태그 이름으로)
```

### gh (GitHub CLI)

- `winget install --id GitHub.cli` 로 설치했다(v2.102.0). 새 터미널을 열어야 PATH 에 잡힌다 — 안
  잡히면 PowerShell 에서 `$env:Path = [Environment]::GetEnvironmentVariable("Path","Machine") + ";" +
  [Environment]::GetEnvironmentVariable("Path","User")` 를 먼저 실행한다.
- 로그인: `gh auth login` → GitHub.com → HTTPS → 브라우저 로그인(`shupil1021`). 이미 되어 있다
  (`gh auth status` 로 확인).
- `assets/movie.mp4`(영상 플레이어용 선택 파일)는 저장소에 없어서 릴리스에도 없다.
