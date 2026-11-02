# Building and distributing AIO Cooler Screen

This repository is a fork of [Thnorty/aio-cooler-screen](https://github.com/Thnorty/aio-cooler-screen). Preserve the original MIT licence, copyright notices and relevant third-party licences when distributing builds.

## GitHub Actions build

The workflow `.github/workflows/build-windows.yml` runs on Windows, installs the Rust MSVC toolchain and Inno Setup, and invokes `build-installer.ps1`. The script builds the Rust application, prepares required PawnIO components and fetches additional fonts and their licence files using `tools/fetch-fonts.ps1`.

On every push to `main`, and on manual dispatch, the workflow generates two downloadable artifacts:

- `aio-cooler-screen-windows-setup`: Full installer, named `AIO-Cooler-Screen-Setup-<version>.exe`.
- `aio-cooler-screen-standalone-exe`: App EXE for advanced use, **not** a complete installation; added fonts are not embedded in this EXE.

To trigger manually with GitHub CLI:

```powershell
gh workflow run build-windows.yml -R SefaMeert/aio-cooler-screen
```

To list runs:

```powershell
gh run list -R SefaMeert/aio-cooler-screen
```
