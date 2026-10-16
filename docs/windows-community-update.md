# Windows community update 1.2.0

Maintained by **@zejio**, based on PdfCraft 0.8.0 at upstream commit
`f0c9901c398372782a4eb001c95638289a5edab0`. This is an independently packaged,
modified development-profile build. The native application keeps its upstream
0.8.0 version; **1.2.0 identifies this Explorer/update package**.

Get the Windows x64 ZIP and the separate, MIT-licensed Explorer helper source from
[the community release](https://github.com/zejio/pdfcraft/releases/tag/explorer-update-v1.2.0).
The helper is distributed separately from this Rust application and uses its
explicitly enabled, authenticated loopback UI control channel. No service or
globally enabled control server is installed.

## Install or update

1. Save your work and close old PdfCraft windows.
2. Extract the entire `PdfCraft-Update-1.2.0-windows-x64.zip`.
3. Run `PdfCraftExplorerAddon.exe` and accept installation for your Windows account.
4. Select images or PDFs in Explorer and right-click. On Windows 11, use **Show more options**.
5. Open the updated app through these menus or **Start > PdfCraft updated**.

The package includes the patched app, its runtime DLL, independent menu icons and
license notices. It does not require a previous PdfCraft installation or administrator
rights. It installs under `%LOCALAPPDATA%\PdfCraftExplorerAddon\1.2.0`; the original
Program Files app remains available. Its portable settings/recovery data are under
the updated app's `PdfCraftData` directory. Existing windows keep their previous code.

## Changes

- **Convert** opens images as unsaved native documents; Save and Print are chosen in the app.
- **Combine** stages the selected original files in the native workspace, preserving names,
  extensions and modification times. Sort by **Modified**, Combine, then Print without Save.
- Windows **Printer Properties** waits for the driver window to close and reads back supported
  paper, color, duplex, copies, collation and orientation settings before printing.
- Print preview preserves color unless **Print in grayscale** is selected. Switching back to
  color restores the original image instead of reusing a grayscale texture.

Supported images: PNG, JPEG, TIFF, GIF, BMP and JPEG 2000; Combine also accepts PDFs.
Office conversion, OCR models and the CLI are not included in this package.

## Verification and limits

Synthetic control, print-library and preview regressions passed. Original-file intake,
sorting in both directions, unsaved Combine to Print, Convert, real Explorer invocation
with an old bridge loaded, installation, repeat installation and the extracted ZIP were
checked on Windows. Packaged preview screenshots were inspected for color/gray/color.

Read-only FUJIFILM Apeos C3070 PS checks resolved its vendor A4 form and validated color
and monochrome settings. The selected Manual Feed bin survived color-ticket validation.
No automated print job was submitted; the user confirmed physical color output. Other
printers, all trays and every physical preset have not all been verified.

The build is unsigned, with debug symbols removed and the executable set to the Windows
GUI subsystem. It bundles `libunwind.dll` from LLVM MinGW UCRT 20261006 with its license.
Dependency/asset notices and per-file SHA-256 checksums are included. No user documents,
settings, recovery files, screenshots, private paths or control tokens are distributed.

Use Windows Installed apps > **PdfCraft Explorer add-on** to remove menu registration and
the Start shortcut. Application and recovery files are retained to protect unsaved work.

## Source and rebuild

The native source corresponding to the package is the release tag in this repository.
`PdfCraft-Explorer-Source-1.2.0.zip` contains the independent C# helper, its tests and build/
packaging scripts, including `BUILD.txt`. It builds using Windows' .NET Framework compiler.
The native build used Rust 1.99.1 `x86_64-pc-windows-gnullvm` and LLVM MinGW UCRT 20261006:

```text
cargo --config profile.dev.package.pdfcraft-engine.opt-level=0 --config profile.dev.package.pdfcraft-ui-egui.opt-level=0 build --locked -p pdfcraft -j 2
```

The modified app's UI omits the ArtCraft trademark logos. Native code remains
MIT OR Apache-2.0; the independent Explorer helper and original geometric menu icons are MIT.
