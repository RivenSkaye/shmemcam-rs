# Shared Memory Camera as a Windows Service

This is a Windows Service that takes all available cameras exposed through the Miscrosoft Multimedia Framework (MSMF) and captures them for as long as it runs.
Once it does, it exposes their image captures over shared memory, powered by [WinMMF](https://crates.io/crates/winmmf).

Over time, this might also support Linux; Considering shmem is much more simple there, it should be easier to implement and for others to use.
