# Shared Memory Camera as a Windows Service

This is a Windows Service that takes all available cameras exposed through the Miscrosoft Multimedia Framework (MSMF) and captures them for as long as it runs.
Once it does, it exposes their image captures over shared memory, powered by [WinMMF](https://crates.io/crates/winmmf).
