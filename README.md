# SC:R Multi-Launcher

A GUI application to help you run multiple instances of StarCraft: Remastered. It operates in a Windows environment (Windows 7 or higher). This project was inspired by and references the C++ repository [sc_multiloader](https://github.com/somersby10ml/sc_multiloader) and the Rust repository [SC1-Multi-Launcher](https://github.com/Alfex4936/SC1-Multi-Launcher).

## Key Features

- **Multiple Client Execution**: Allows running several StarCraft: Remastered clients simultaneously.
- **GUI-Based**: Provides an intuitive graphical interface using the `iced` framework.
- **Batch Process Termination**: The 'Kill All' button terminates all running StarCraft processes at once.
- **Automatic Process Detection**: Periodically detects and manages running StarCraft processes.
- **Logging**: Records and saves logs for key operations like client launches, terminations, and handle manipulations.
- **Easy Configuration**: Easily set the path for the StarCraft executable (`StarCraft.exe`) and save it to `conf.toml`.

## How It Works

StarCraft prevents multiple instances from running simultaneously by using a mutex named "Starcraft Check For Other Instances". This application works by detecting any running StarCraft process, finding the specific mutex handle within that process, and closing it. This procedure tricks the game into thinking no other instances are active, thereby allowing multiple clients to launch.

## Usage

1. **Initial Setup**:

   - When you first run the program, the 32-bit and 64-bit buttons will be disabled.
   - Click the gear icon (⚙️) at the top to open the settings window.
   - In the '32bit' and '64bit' sections, specify the path to your `StarCraft.exe` file for each architecture. (Clicking the folder icon will open a file dialog.)
   - Click 'OK' to save the settings. The configuration will be saved in `conf.toml` in the same directory as the executable.

2. **Launching a Client**:

   - On the main screen, click the `32bit` or `64bit` button to launch the desired version of StarCraft.

3. **Process Management**:
   - **Log Window**: The central white area displays real-time logs for operations like process launches, terminations, and mutex handle closures.
   - **Kill All**: Immediately terminates all running StarCraft clients.
   - **Save Logs**: Saves the current logs to a file named `YYYY-MM-DD.txt` and then clears the log window.
   - **Clear Logs**: Clears all logs from the screen.

## Configuration File (`conf.toml`)

The application settings are stored in `conf.toml`. You can also edit this file directly.

```toml
# Path to the 32-bit StarCraft.exe
path_32 = "C:\\Program Files (x86)\\StarCraft\\x86\\StarCraft.exe"
# Path to the 64-bit StarCraft.exe
path_64 = "C:\\Program Files (x86)\\StarCraft\\x86_64\\StarCraft.exe"
```

## Building from Source

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)

### Build Command

Run the following command in the project root directory:

```sh
cargo build --release
```

Once the build is complete, the executable will be available at `target/release/scr-multi-launcher.exe`.

## Key Dependencies

- [iced](https://github.com/iced-rs/iced): GUI library
- [tokio](https://github.com/tokio-rs/tokio): Asynchronous runtime
- [windows-rs](https://github.com/microsoft/windows-rs): Windows API bindings
- [serde](https://github.com/serde-rs/serde) & [toml](https://github.com/toml-rs/toml): Serialization/deserialization for the config file

---

<details>
<summary>**한국어 (Korean)**</summary>

# SC:R Multi-Launcher

StarCraft: Remastered를 여러 개 실행할 수 있도록 도와주는 GUI 애플리케이션입니다. Windows 7 이상 환경에서 작동합니다. 이 프로젝트는 C++ 리포지토리 [sc_multiloader](https://github.com/somersby10ml/sc_multiloader)와 Rust 리포지토리 [SC1-Multi-Launcher](https://github.com/Alfex4936/SC1-Multi-Launcher)를 참고하여 제작되었습니다.

## 주요 기능

- **다중 클라이언트 실행**: StarCraft: Remastered 클라이언트를 여러 개 실행할 수 있습니다.
- **GUI 기반**: `iced` 프레임워크를 사용하여 직관적인 그래픽 인터페이스를 제공합니다.
- **프로세스 일괄 종료**: 'Kill All' 버튼으로 실행 중인 모든 스타크래프트 프로세스를 한 번에 종료할 수 있습니다.
- **자동 프로세스 감지**: 실행 중인 스타크래프트 프로세스를 주기적으로 감지하고 관리합니다.
- **로그 기능**: 클라이언트 실행, 종료, 핸들 조작 등 주요 작업에 대한 로그를 기록하고 파일로 저장할 수 있습니다.
- **간편한 설정**: 스타크래프트 실행 파일(`StarCraft.exe`)의 경로를 쉽게 설정하고 `conf.toml` 파일에 저장합니다.

## 원리

Starcraft는 중복 실행을 방지하기 위해 "Starcraft Check For Other Instances"라는 이름의 뮤텍스(Mutex) 핸들을 사용합니다. 이 애플리케이션은 스타크래프트 프로세스가 탐지되면, 해당 프로세스의 이 뮤텍스 핸들을 찾아 종료하여 중복 실행을 가능하게 만듭니다.

## 사용법

1. **최초 설정**:

   - 프로그램을 처음 실행하면 32비트 및 64비트 버튼이 비활성화되어 있습니다.
   - 상단의 톱니바퀴 모양 설정 버튼(⚙️)을 클릭하여 설정 창을 엽니다.
   - '32bit' 및 '64bit' 섹션에서 각각의 `StarCraft.exe` 파일 경로를 지정해 줍니다. (폴더 아이콘 버튼을 누르면 파일 탐색기가 열립니다.)
   - '확인'을 눌러 설정을 저장합니다. 설정은 실행 파일과 동일한 경로에 `conf.toml` 파일로 저장됩니다.

2. **클라이언트 실행**:

   - 메인 화면의 `32bit` 또는 `64bit` 버튼을 눌러 원하는 버전의 스타크래프트를 실행합니다.

3. **프로세스 관리**:
   - **로그 영역**: 중앙의 흰색 영역에는 스타크래프트 프로세스 실행, 종료, 뮤텍스 핸들 닫기 등의 작업 로그가 실시간으로 표시됩니다.
   - **Kill All**: 실행 중인 모든 스타크래프트 클라이언트를 즉시 종료합니다.
   - **Save Logs**: 현재까지의 로그를 `YYYY-MM-DD.txt` 형식의 파일로 저장합니다. 화면의 로그는 지워집니다.
   - **Clear Logs**: 화면의 로그를 모두 지웁니다.

## 설정 파일 (`conf.toml`)

애플리케이션 설정은 `conf.toml` 파일에 저장됩니다. 직접 편집할 수도 있습니다.

```toml
# 32비트 StarCraft.exe 경로
path_32 = "C:\\Program Files (x86)\\StarCraft\\x86\\StarCraft.exe"
# 64비트 StarCraft.exe 경로
path_64 = "C:\\Program Files (x86)\\StarCraft\\x86_64\\StarCraft.exe"
```

## 소스에서 빌드하기

### 요구 사항

- [Rust](https://www.rust-lang.org/tools/install)

### 빌드 명령어

프로젝트 루트 디렉토리에서 다음 명령어를 실행합니다.

```sh
cargo build --release
```

빌드가 완료되면 `target/release/scr-multi-launcher.exe` 실행 파일이 생성됩니다.

## 주요 의존성

- [iced](https://github.com/iced-rs/iced): GUI 라이브러리
- [tokio](https://github.com/tokio-rs/tokio): 비동기 런타임
- [windows-rs](https://github.com/microsoft/windows-rs): Windows API 바인딩
- [serde](https://github.com/serde-rs/serde) & [toml](https://github.com/toml-rs/toml): 설정 파일 직렬화/역직렬화

</details>
