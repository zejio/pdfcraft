//! Read back the settings committed by Windows' driver preferences before laying out a job.
//! The maintained winprint wrapper owns the native calls; parsing its DEVMODE needs no unsafe.

use super::Duplex;
use crate::PrintError;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DriverSettings {
    pub paper: Option<(f64, f64)>,
    pub grayscale: Option<bool>,
    pub duplex: Option<Duplex>,
    pub copies: Option<u32>,
    pub collate: Option<bool>,
    pub landscape: Option<bool>,
}

/// Open Windows' existing preferences UI, wait until it closes, then read this user's defaults.
/// Run on a worker: neither a driver dialog nor a network printer may block the application's UI.
pub fn edit_printer_preferences(printer: &str) -> Result<DriverSettings, PrintError> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;
        if printer.is_empty() || printer.contains(['"', '\0']) {
            return Err(error("not a printer name"));
        }
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        let status = std::process::Command::new(std::path::Path::new(&root).join("System32/rundll32.exe"))
            .raw_arg("printui.dll,PrintUIEntry /e /n")
            .raw_arg(format!("\"{printer}\""))
            .creation_flags(0x08000000)
            .status()
            .map_err(error)?;
        if !status.success() {
            return Err(error("the printer's preferences did not finish successfully"));
        }
        read_printer_preferences(printer)
    }
    #[cfg(not(windows))]
    {
        let _ = printer;
        Err(error("native printer preferences are available only on Windows"))
    }
}

/// Read only; never changes a printer or sends a job. The empty ticket asks winprint to start
/// from kUserDefaultDevmode, including the user's saved driver preset, rather than app defaults.
pub fn read_printer_preferences(printer: &str) -> Result<DriverSettings, PrintError> {
    #[cfg(windows)]
    {
        let device = winprint::printer::PrinterDevice::all()
            .map_err(error)?
            .into_iter()
            .find(|device| device.name() == printer)
            .ok_or_else(|| error("the selected printer is no longer available"))?;
        let bytes = winprint::ticket::PrintTicket::default().to_dev_mode(&device).map_err(error)?;
        parse_devmode(&bytes)
    }
    #[cfg(not(windows))]
    {
        let _ = printer;
        Err(error("native printer preferences are available only on Windows"))
    }
}

fn error(message: impl std::fmt::Display) -> PrintError {
    PrintError::Spool(message.to_string())
}

/// DEVMODEW's public, fixed-width fields (wingdi.h). Driver-private bytes are never interpreted.
#[cfg(any(windows, test))]
fn parse_devmode(bytes: &[u8]) -> Result<DriverSettings, PrintError> {
    let word = |offset: usize| -> Result<u16, PrintError> {
        let value = bytes.get(offset..offset.saturating_add(2)).ok_or_else(|| error("truncated printer settings"))?;
        Ok(u16::from_le_bytes([value[0], value[1]]))
    };
    let public = usize::from(word(68)?);
    let total = public.checked_add(usize::from(word(70)?)).ok_or_else(|| error("invalid printer settings length"))?;
    if public < 102 || total > bytes.len() {
        return Err(error("truncated printer settings"));
    }
    let fields = u32::from(word(72)?) | (u32::from(word(74)?) << 16);
    let field = |flag, offset| -> Result<Option<u16>, PrintError> { if fields & flag == 0 { Ok(None) } else { word(offset).map(Some) } };
    let paper = if fields & 0x0c == 0x0c {
        let (width, length) = (word(82)?, word(80)?);
        if width == 0 || length == 0 || width > i16::MAX as u16 || length > i16::MAX as u16 {
            return Err(error("invalid printer paper dimensions"));
        }
        Some((f64::from(width) * 72.0 / 254.0, f64::from(length) * 72.0 / 254.0))
    } else {
        match field(2, 78)? {
            Some(1 | 2 | 18) => Some((612.0, 792.0)),
            Some(3 | 4 | 17) => Some((792.0, 1224.0)),
            Some(5) => Some((612.0, 1008.0)),
            Some(8) => Some((841.89, 1190.55)),
            Some(9 | 10) => Some((595.28, 841.89)),
            Some(11) => Some((419.53, 595.28)),
            Some(70) => Some((297.64, 419.53)),
            Some(24) => Some((1224.0, 1584.0)),
            Some(25) => Some((1584.0, 2448.0)),
            Some(26) => Some((2448.0, 3168.0)),
            Some(66) => Some((1190.55, 1683.78)),
            Some(_) if fields & 0x10000 != 0 && public >= 166 => {
                let form = (102..166).step_by(2).map(word).collect::<Result<Vec<_>, _>>()?;
                let form = String::from_utf16_lossy(form.split(|c| *c == 0).next().unwrap_or_default());
                // Some PS drivers use vendor paper IDs even for A4 (e.g. Apeos: 129).
                // Accept a standard form name, optionally followed by its dimensions,
                // without confusing A4 Extra or a similarly prefixed custom form with A4.
                let paper = crate::PAPERS
                    .iter()
                    .find(|(name, _)| {
                        let name = match *name {
                            "US Letter" => "Letter",
                            "US Legal" => "Legal",
                            other => other,
                        };
                        form.get(..name.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(name))
                            && form.get(name.len()..).is_some_and(|suffix| {
                                let suffix = suffix.trim_start();
                                suffix.is_empty() || suffix.starts_with('(') || suffix.starts_with(|c: char| c.is_ascii_digit())
                            })
                    })
                    .map(|(_, size)| *size);
                Some(paper.ok_or_else(|| error(format!("unsupported driver paper form {form:?}; choose a supported size in Properties")))?)
            }
            Some(_) => return Err(error("the driver selected an unsupported paper size; choose a supported size in Properties")),
            None => None,
        }
    };
    let grayscale = match field(0x800, 92)? {
        Some(1) => Some(true),
        Some(2) => Some(false),
        None => None,
        _ => return Err(error("invalid printer color setting")),
    };
    let duplex = match field(0x1000, 94)? {
        Some(1) => Some(Duplex::Off),
        Some(2) => Some(Duplex::LongEdge),
        Some(3) => Some(Duplex::ShortEdge),
        None => None,
        _ => return Err(error("invalid printer duplex setting")),
    };
    let landscape = match field(1, 76)? {
        Some(1) => Some(false),
        Some(2) => Some(true),
        None => None,
        _ => return Err(error("invalid printer orientation setting")),
    };
    Ok(DriverSettings {
        paper,
        grayscale,
        duplex,
        copies: field(0x100, 86)?.filter(|copies| (1..=999).contains(copies)).map(u32::from),
        collate: field(0x8000, 100)?.map(|value| value == 1),
        landscape,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a4_monochrome_preset_and_rejects_truncated_devmodes() {
        let mut bytes = vec![0; 228];
        for (offset, value) in [(68, 220u16), (70, 8), (72, 0x9903), (76, 1), (78, 9), (86, 2), (92, 1), (94, 2), (100, 1)] {
            bytes.get_mut(offset..offset + 2).unwrap().copy_from_slice(&value.to_le_bytes());
        }
        let settings = parse_devmode(&bytes).unwrap();
        assert_eq!(settings.paper, Some((595.28, 841.89)));
        assert_eq!(settings.grayscale, Some(true));
        assert_eq!(settings.duplex, Some(Duplex::LongEdge));
        assert_eq!(settings.copies, Some(2));
        assert_eq!(settings.collate, Some(true));
        assert_eq!(settings.landscape, Some(false));
        for len in 0..bytes.len() {
            assert!(parse_devmode(bytes.get(..len).unwrap()).is_err());
        }
        bytes.get_mut(92..94).unwrap().copy_from_slice(&2u16.to_le_bytes());
        assert_eq!(parse_devmode(&bytes).unwrap().grayscale, Some(false));
        bytes.get_mut(74..76).unwrap().copy_from_slice(&1u16.to_le_bytes());
        bytes.get_mut(78..80).unwrap().copy_from_slice(&129u16.to_le_bytes());
        for (i, value) in "A4 (210 x 297mm)".encode_utf16().enumerate() {
            bytes.get_mut(102 + i * 2..104 + i * 2).unwrap().copy_from_slice(&value.to_le_bytes());
        }
        assert_eq!(parse_devmode(&bytes).unwrap().paper, Some(crate::PAPERS[4].1), "Apeos PS vendor paper ID");
    }
}
