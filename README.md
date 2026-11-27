# Ratatui Diary

Ratatui Diary는 Rust와 Ratatui로 만든 터미널 일기장입니다. 월간 달력에서 날짜를 골라 일기를 열고, 본문을 편집하면서 옆에서 미리보기를 볼 수 있습니다. 새 일기는 Org 파일로 저장하며 기존 Markdown 일기도 열고 편집할 수 있습니다.

- **달력과 미리보기** — 일기가 있는 날짜를 표시하고, 선택한 날짜의 일기를 달력 옆에 보여 줍니다.
- **두 가지 편집 방식** — 기본 Emacs 키맵과 단일 선택 기반의 Helix 키맵을 지원합니다. 현재 화면과 입력 상태에 맞는 단축키가 하단에 표시됩니다.
- **전체 일기 검색** — 저장된 일기에서 문자열이나 의미가 비슷한 구절을 찾아 원문 위치로 이동합니다.
- **화면 설정** — 한국어·영어 UI와 시스템·다크·라이트 테마를 지원합니다.

## 빠른 시작

Rustup과 플랫폼의 네이티브 빌드 도구가 필요합니다. 저장소 루트에서 Cargo를 실행하면 [rust-toolchain.toml](rust-toolchain.toml)에 지정된 Rust nightly를 사용합니다.

```sh
cargo run --locked
```

달력에서 방향키로 날짜를 고르고 `Enter`를 눌러 일기를 작성합니다. 기본 Emacs 키맵에서는 `Ctrl+X`를 누른 뒤 `Ctrl+S`로 저장하고, `Ctrl+X`를 누른 뒤 `k`로 저장 후 달력에 돌아갑니다. 달력에서 `/`를 누르면 전체 일기를 검색할 수 있습니다.

PATH에 추가된 Cargo 바이너리 디렉터리에 설치해 어느 디렉터리에서든 실행할 수 있습니다.

```sh
cargo install --locked --path crates/ratatui-diary
ratatui-diary
```

## 사용법

### 달력

| 동작 | 공통 | Emacs | Helix |
| --- | --- | --- | --- |
| 날짜·주 이동 | 방향키 | `Ctrl+B/F/P/N` | `h/j/k/l` |
| 이전·다음 달 | `PageUp` / `PageDown` | `Alt+P` / `Alt+N` | `H` / `L` |
| 이전·다음 해 | `Ctrl+PageUp` / `Ctrl+PageDown` | `Alt+[` / `Alt+]` | `{` / `}` |
| 일기 작성·편집 | `Enter` | | |
| 전체 일기 검색 | `/` | | |
| 앱 종료 | | `Ctrl+Q` 또는 `Ctrl+X` → `Ctrl+C` | `:q` → `Enter` |

월·연도 이동 단축키는 가능한 한 같은 일자를 유지합니다. 해당 월에 그 일자가 없으면 마지막 날을 선택합니다.

### 에디터 화면 (Emacs 스타일)

문자를 입력하면 커서 위치에 바로 삽입됩니다. 아래 표에서 화살표는 키를 순서대로 누른다는 뜻입니다.

| 키 | 동작 |
| --- | --- |
| 방향키 또는 `Ctrl+B/F/P/N` | 커서 이동 |
| `Ctrl+A` / `Ctrl+E` | 줄 시작 / 끝으로 이동 |
| `Ctrl+Space`, 이후 커서 이동 | 영역 선택 |
| `Ctrl+W` / `Alt+W` / `Ctrl+Y` | 선택 잘라내기 / 복사 / 붙여넣기 |
| `Ctrl+/` / `Ctrl+Shift+Z` | 실행 취소 / 다시 실행 |
| `Ctrl+S` / `Ctrl+R` | 현재 일기에서 정방향 / 역방향 증분 검색 |
| `Ctrl+X` → `Ctrl+S` 또는 `s` | 저장 |
| `Ctrl+X` → `k` | 저장 후 달력으로 돌아가기 |
| `Ctrl+X` → `Ctrl+C` 또는 `Ctrl+Q` | 변경 사항을 저장하고 종료 |
| `Ctrl+G` | 선택 해제, 접두키 취소 또는 검색 취소 |

증분 검색 중에는 `Ctrl+S`와 `Ctrl+R`로 다음·이전 결과를 찾습니다. `Enter`로 확정하고 `Ctrl+G` 또는 `Esc`로 검색 전 위치에 돌아갑니다.

### 에디터 화면 (Helix)

Normal 모드에서 시작합니다. 이동으로 범위를 선택하고 `d`나 `c`로 편집합니다. `v`로 Select 모드에 들어가면 이동할 때 선택이 확장됩니다.

| 키 | 동작 |
| --- | --- |
| `h/j/k/l`, `w/b/e` | 글자·줄 단위 또는 단어 단위 이동 |
| `i` / `a` | 선택 앞 / 뒤에서 Insert 모드 진입 |
| `Esc` | Normal 모드로 복귀하거나 진행 중인 명령 취소 |
| `d` / `c` / `y` | 선택 삭제 / 삭제 후 Insert 모드 / 복사 |
| `p` / `P` | 선택 뒤 / 앞에 붙여넣기 |
| `u` / `U` | 실행 취소 / 다시 실행 |
| `/` / `?`, `n` / `N` | 정방향·역방향 정규식 검색, 다음·이전 결과 선택 |
| `:w`, `:wq` | 저장, 저장 후 종료 |
| `Space` → `f` | 변경 사항을 저장하고 달력으로 돌아가기 |
| `:bc`, `:q` | 달력으로 돌아가기, 종료. 저장하지 않은 변경 사항이 있으면 중단 |
| `:q!` | 저장하지 않은 변경 사항을 버리고 종료 |

콜론 명령은 `Enter`로 실행합니다. 검색은 검색어에 대문자가 있을 때만 대소문자를 구분합니다. 두 키맵 모두 저장에 실패하면 편집 내용을 유지하고, 예정된 종료나 달력 복귀를 중단합니다.

### 전체 일기 검색

1. 달력에서 `/`를 누르고 검색어를 입력한 뒤 `Enter`를 누릅니다.
2. 결과를 방향키로 고릅니다. `Tab`으로 검색어 입력과 결과 선택을 전환합니다.
3. 결과에서 `Enter`를 누르면 해당 일기의 구절로 이동합니다. `Esc`로 달력에 돌아갑니다.

검색은 대소문자를 구분하지 않는 문자열 일치를 우선 표시하고 의미가 비슷한 구절도 함께 보여 줍니다. 의미 검색은 CPU에서 multilingual-e5-small 모델을 실행합니다. 앱은 첫 화면을 그린 뒤 모델을 백그라운드에서 준비하며, 처음에는 공개 모델 파일을 받기 위해 인터넷 연결이 필요합니다. 이후에는 캐시된 모델로 오프라인 검색을 할 수 있습니다.

일기 본문과 검색어는 로컬에서 처리합니다. 모델을 불러오거나 실행하는 데 실패하면 문자열 결과를 표시합니다. 검색 엔진과 측정 방법은 [검색 성능 문서](docs/search-performance.md)를 참고하세요.

### 설정

`F2`로 설정을 열고 `↑` / `↓`로 항목을 선택한 뒤 `←` / `→`로 값을 바꿉니다. 기본값은 **한국어**, **시스템 테마**, **Emacs 키맵**입니다. 시스템 테마는 터미널의 기본 색상을 사용합니다. 설정은 변경 즉시 적용되고 자동으로 저장됩니다.

## 일기와 설정 파일

| 운영체제 | 일기 디렉터리 | 설정 파일 |
| --- | --- | --- |
| Linux | `$XDG_DATA_HOME/ratatui-diary/entries/` (기본 `~/.local/share/ratatui-diary/entries/`) | `$XDG_CONFIG_HOME/ratatui-diary/settings.conf` (기본 `~/.config/ratatui-diary/settings.conf`) |
| macOS | `~/Library/Application Support/ratatui-diary/entries/` | `~/Library/Application Support/ratatui-diary/settings.conf` |
| Windows | `%LOCALAPPDATA%\ratatui-diary\entries\` | `%APPDATA%\ratatui-diary\settings.conf` |

새 일기는 UTF-8 텍스트 파일 `YYYY-MM-DD.org`로 저장합니다. 줄 맨 앞의 `* 제목`이나 `** 하위 제목`은 미리보기에서 깊이별 불릿으로 표시됩니다. 기존 `YYYY-MM-DD.md`는 Markdown으로 미리보고 같은 파일에 저장합니다. 두 파일이 같은 날짜에 있으면 `.org`를 우선합니다.

일기는 파일당 4 MiB, 설정은 64 KiB까지 읽고 쓸 수 있습니다. 일기와 설정을 저장할 때는 임시 파일에 기록한 뒤 기존 파일을 교체합니다.

임베딩 캐시는 일기 디렉터리의 `.search-cache/`에 저장되며 필요할 때 다시 생성됩니다. 검색 모델은 OS 캐시 디렉터리의 `ratatui-diary/models/`에 저장됩니다. `HF_HOME`을 설정하면 모델에 그 경로를 사용합니다.

## 개발

```sh
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked
```

### Workspace 구조

| 크레이트 | 책임 |
| --- | --- |
| `diary-domain` | 달력의 날짜 규칙, 일기와 설정 값, 검색 결과의 본문 위치 확인 |
| `text-editor` | 편집 버퍼, 커서, 선택, 실행 취소 |
| `diary-application` | 일기 열기·저장 후 이동, 설정 변경, 검색 세션의 상태 전이와 키맵 |
| `diary-storage` | 일기, 설정, 임베딩 캐시의 파일 읽기·쓰기 |
| `diary-search` | 검색 워커, 모델 로드, 색인 및 순위 계산 |
| `diary-runtime` | 파일과 검색 어댑터 조합, 명령 실행 및 결과 전달 |
| `ratatui-diary`, `org-render`, `markdown-render` | 터미널 입력·출력과 미리보기 |

애플리케이션은 도메인과 편집 엔진을 사용합니다. 런타임이 파일·검색 어댑터를 조합하고, 터미널은 키 입력, 현재 날짜, 화면 크기를 애플리케이션의 입력 값으로 전달합니다.

```text
Model + Msg → transition → Model + Option<Command>
Command     → Runtime::perform → Option<Msg>
```

`transition`은 모델의 소유권을 받아 다음 모델을 반환합니다. 내부에서 기존 버퍼를 재사용하므로 전이마다 일기 전체를 복제할 필요가 없습니다. `Command`는 효과 실행에 필요한 값을 소유하며, 런타임은 실행 결과를 다시 `Msg`로 전달합니다. 파일 작업의 결과는 다음 입력을 받기 전에 모두 적용합니다. 검색은 워커에서 비동기로 실행하며 현재 요청 번호와 일치하는 응답만 적용합니다.

DCI에서 **Data**는 일기, 편집 버퍼, 달력 선택, 설정, 검색 결과입니다. **Context**는 일기 작업, 설정 변경, 검색 세션입니다. **Interaction**은 각 컨텍스트에서 데이터가 맡은 역할 사이의 협력입니다. 예를 들어 일기 열기에서 달력은 날짜를 선택하고, 검색 결과는 구절의 위치를 찾으며, 편집기는 읽어 온 일기와 커서 위치를 적용합니다. 저장 후 돌아가기에서는 저장 성공 시 일기 색인을 갱신하고 달력 미리보기를 요청합니다. 작업 상태는 Rust의 `enum`으로 표현해 대기 중인 날짜나 검색 요청 번호를 해당 상태와 함께 보관합니다.

기본 테스트는 검색 모델을 다운로드하지 않습니다. 실제 모델을 쓰는 한국어 의미 검색 테스트와 편집 경로 벤치마크는 따로 실행할 수 있습니다.

```sh
cargo test --locked -p diary-search real_korean_semantic_search -- --ignored
cargo bench --locked -p ratatui-diary --bench hotpaths
```

### 배포 파일 만들기

배포 스크립트는 기본적으로 포맷 검사, Clippy, 테스트를 실행한 뒤 릴리스 바이너리를 빌드해 `dist/`에 패키징합니다.

- **macOS·Linux** — `bash scripts/release.sh`로 `dist/ratatui-diary-<버전>-<타깃>.tar.gz`를 만듭니다. 옵션은 `bash scripts/release.sh --help`에서 확인할 수 있습니다.
- **Windows** — PowerShell 5.1 이상과 Inno Setup 6으로 `dist/ratatui-diary-<버전>-<아키텍처>-setup.exe`를 만듭니다. Inno Setup이 없으면 Winget으로 설치하는 `-InstallInno` 옵션을 사용할 수 있습니다. [Windows 배포 스크립트](scripts/release.ps1)

```powershell
.\scripts\release.ps1 -InstallInno
```

## 라이선스

Ratatui Diary는 kangnodle의 독점 소프트웨어이며 모든 권리가 유보됩니다. 사용 및 배포 조건은 [LICENSE](LICENSE)를 참조하세요. 제3자 구성 요소에는 각 구성 요소의 라이선스가 적용됩니다.
