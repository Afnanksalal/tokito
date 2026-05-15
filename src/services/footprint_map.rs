//! Map package strings to footprint identifiers.

/// Heuristic mapping from LCSC/Nexar package text to a footprint library name.
pub fn hint_from_package(package: &str) -> String {
    let p = package.trim();
    let upper = p.to_ascii_uppercase();
    if upper.contains("0402") {
        return "Resistor_SMD:R_0402_1005Metric".into();
    }
    if upper.contains("0603") {
        return "Resistor_SMD:R_0603_1608Metric".into();
    }
    if upper.contains("0805") {
        return "Resistor_SMD:R_0805_2012Metric".into();
    }
    if upper.contains("1206") {
        return "Resistor_SMD:R_1206_3216Metric".into();
    }
    if upper.contains("1210") {
        return "Resistor_SMD:R_1210_3225Metric".into();
    }
    if upper.contains("2010") {
        return "Resistor_SMD:R_2010_5025Metric".into();
    }
    if upper.contains("2512") {
        return "Resistor_SMD:R_2512_6332Metric".into();
    }
    if upper.contains("SOT-23-6") || upper.contains("SOT23-6") {
        return "Package_TO_SOT_SMD:SOT-23-6".into();
    }
    if upper.contains("SOT-23-5") || upper.contains("SOT23-5") {
        return "Package_TO_SOT_SMD:SOT-23-5".into();
    }
    if upper.contains("SOT-23") || upper.contains("SOT23") {
        return "Package_TO_SOT_SMD:SOT-23".into();
    }
    if upper.contains("SOIC-16") {
        return "Package_SO:SOIC-16_3.9x9.9mm_P1.27mm".into();
    }
    if upper.contains("SOIC-14") {
        return "Package_SO:SOIC-14_3.9x8.7mm_P1.27mm".into();
    }
    if upper.contains("SOIC-8") {
        return "Package_SO:SOIC-8_3.9x4.9mm_P1.27mm".into();
    }
    if upper.contains("TSSOP-20") {
        return "Package_SO:TSSOP-20_4.4x6.5mm_P0.65mm".into();
    }
    if upper.contains("TSSOP-16") {
        return "Package_SO:TSSOP-16_4.4x5mm_P0.65mm".into();
    }
    if upper.contains("QFP-64") || upper.contains("LQFP-64") {
        return "Package_QFP:LQFP-64_10x10mm_P0.5mm".into();
    }
    if upper.contains("QFP-48") || upper.contains("LQFP-48") {
        return "Package_QFP:LQFP-48_7x7mm_P0.5mm".into();
    }
    if upper.contains("QFP-32") || upper.contains("LQFP-32") {
        return "Package_QFP:LQFP-32_7x7mm_P0.8mm".into();
    }
    if upper.contains("TO-252") || upper.contains("DPAK") {
        return "Package_TO_SOT_SMD:TO-252-2".into();
    }
    if upper.contains("TO-220") {
        return "Package_TO_SOT_THT:TO-220-3_Vertical".into();
    }
    if upper.contains("TO-92") {
        return "Package_TO_SOT_THT:TO-92_Inline".into();
    }
    if upper.contains("DO-214") || upper.contains("SMA") {
        return "Diode_SMD:D_SMA".into();
    }
    if upper.contains("SOD-123") {
        return "Diode_SMD:D_SOD-123".into();
    }
    if upper.contains("SOD-323") {
        return "Diode_SMD:D_SOD-323".into();
    }
    if upper.contains("HC-49") || upper.contains("HC49") {
        return "Crystal:Crystal_HC49-4H_Vertical".into();
    }
    if upper.contains("3225") {
        return "Crystal:Crystal_SMD_3225-4Pin_3.2x2.5mm".into();
    }
    format!("Package_Custom:{p}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_sot23() {
        assert!(hint_from_package("SOT-23-5").contains("SOT-23-5"));
    }

    #[test]
    fn maps_0805() {
        assert!(hint_from_package("0805").contains("0805"));
    }
}
