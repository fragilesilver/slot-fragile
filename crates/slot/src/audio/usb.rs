pub fn present_usb_card() -> Option<u32> {
    let cards = std::fs::read_to_string(slot_power::SOUND_CARDS).ok()?;
    slot_power::usb_card(&cards)
}

pub fn devices(route: Option<&str>, usb: Option<u32>, builtin: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(bt) = route {
        out.push(bt.to_string());
    }
    if let Some(card) = usb {
        out.push(format!("plughw:{card},0"));
        out.push(format!("hw:{card},0"));
    }
    for d in builtin {
        out.push((*d).to_string());
    }
    out
}
