//! Read-only driver diagnostic: cargo run -p pdfcraft-print --example printer_preferences -- "queue name"
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let printer = std::env::args().nth(1).ok_or("supply the exact printer name")?;
    println!("{:?}", pdfcraft_print::spool::read_printer_preferences(&printer)?);
    #[cfg(windows)]
    {
        use winprint::ticket::document::{NS_PSK, OwnedName, PrintTicketDocument, reader::ParsableXmlDocument as _};
        let device = winprint::printer::PrinterDevice::all()?.into_iter().find(|device| device.name() == printer).ok_or("printer not found")?;
        let capabilities = winprint::ticket::PrintCapabilities::fetch(&device)?;
        let colors: Vec<_> = capabilities
            .options_for_feature(OwnedName::qualified("PageOutputColor", NS_PSK, Some("psk")))
            .filter_map(|option| option.name.as_ref().map(|name| name.local_name.clone()))
            .collect();
        println!("Driver color options: {colors:?}");
        let requested = if std::env::args().nth(2).as_deref() == Some("color") { "Color" } else { "Monochrome" };
        let defaults = winprint::ticket::PrintTicket::default().to_dev_mode(&device)?;
        let mut ticket = winprint::ticket::PrintTicketBuilder::new(&device)?;
        ticket.merge(winprint::ticket::PrintTicket::from_xml(format!(r#"<psf:PrintTicket version="1" xmlns:psf="http://schemas.microsoft.com/windows/2003/08/printing/printschemaframework" xmlns:psk="http://schemas.microsoft.com/windows/2003/08/printing/printschemakeywords"><psf:Feature name="psk:PageOutputColor"><psf:Option name="psk:{requested}"/></psf:Feature></psf:PrintTicket>"#)))?;
        let ticket = ticket.build()?;
        let accepted = ticket.to_dev_mode(&device)?;
        let bin = |bytes: &[u8]| -> Option<u16> { Some(u16::from_le_bytes(bytes.get(88..90)?.try_into().ok()?)) };
        println!("Paper source: user default {:?}, validated ticket {:?}", bin(&defaults), bin(&accepted));
        let document = PrintTicketDocument::parse_from_bytes(ticket.get_xml())?;
        let color = document
            .features
            .iter()
            .find(|feature| feature.name.namespace_ref() == Some(NS_PSK) && feature.name.local_name == "PageOutputColor")
            .and_then(|feature| feature.options.first())
            .and_then(|option| option.name.as_ref())
            .map(|name| name.local_name.as_str());
        if color != Some(requested) {
            return Err(format!("driver changed requested {requested} to {color:?}").into());
        }
        println!("Driver validated {requested} request: {color:?} (no job submitted)");
    }
    Ok(())
}
