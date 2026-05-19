//! Generate bundled `.tokito_sym` files.

use std::fs;
use std::path::Path;

const HEADER: &str = r#"(tokito_symbol_lib
	(version 20251024)
	(generator "tokito_symbol_gen")
	(generator_version "1.0")
"#;

const FOOTER: &str = ")\n";

pub fn generate_library(out_dir: &Path) -> anyhow::Result<usize> {
    fs::create_dir_all(out_dir)?;
    let mut count = 0usize;

    let smd_passives = [
        ("R", "Resistor", "R_*", (-1.016, 2.54)),
        ("C", "Capacitor", "C_*", (-1.27, 2.54)),
        ("L", "Inductor", "L_*", (-1.0, 2.5)),
    ];
    for suffix in ["0402", "0603", "0805", "1206", "1210", "2010", "2512"] {
        for (sym, desc, fp, (hw, hh)) in smd_passives {
            let name = format!("{sym}_{suffix}");
            write_file(
                &out_dir.join("Device").join(format!("{name}.tokito_sym")),
                &two_pin_passive(&name, desc, fp, hw, hh),
            )?;
            count += 1;
        }
    }

    for n in 1..=20 {
        let name = format!("Conn_01x{n:02}");
        write_file(
            &out_dir
                .join("Connector_Generic")
                .join(format!("{name}.tokito_sym")),
            &connector_1xn(&name, n),
        )?;
        count += 1;
    }

    for (name, pins, desc) in [
        ("74HC00", 14, "Quad 2-input NAND"),
        ("74HC04", 14, "Hex inverter"),
        ("74HC08", 14, "Quad 2-input AND"),
        ("74HC32", 14, "Quad 2-input OR"),
        ("74HC86", 14, "Quad 2-input XOR"),
        ("74HC595", 16, "Shift register"),
        ("74HC165", 16, "Shift register in"),
    ] {
        write_file(
            &out_dir
                .join("Logic_74xx")
                .join(format!("{name}.tokito_sym")),
            &dual_row_ic(name, pins, desc, "U"),
        )?;
        count += 1;
    }

    for (name, pins, desc) in [
        ("STM32F103C8", 48, "MCU LQFP-48"),
        ("STM32F407VG", 100, "MCU LQFP-100"),
        ("ATmega328P", 28, "MCU TQFP-32 class"),
        ("ESP32-WROOM", 38, "WiFi module"),
        ("RP2040", 56, "MCU QFN-56"),
    ] {
        write_file(
            &out_dir.join("MCU").join(format!("{name}.tokito_sym")),
            &dual_row_ic(name, pins.min(64), desc, "U"),
        )?;
        count += 1;
    }

    for (name, desc, fp) in [
        ("AMS1117-3.3", "LDO 3.3V", "SOT-223"),
        ("AMS1117-5.0", "LDO 5V", "SOT-223"),
        ("LM317", "Adjustable regulator", "TO-220"),
        ("MP1584", "Buck converter", "SOIC-8"),
        ("TPS5430", "Buck converter", "SOIC-8"),
    ] {
        write_file(
            &out_dir
                .join("Regulator_Switching")
                .join(format!("{name}.tokito_sym")),
            &regulator_3pin(name, desc, fp),
        )?;
        count += 1;
    }

    for (name, desc) in [
        ("SW_SPST", "SPST switch"),
        ("SW_Push", "Push button"),
        ("SW_DIP4", "DIP switch 4"),
    ] {
        write_file(
            &out_dir.join("Switch").join(format!("{name}.tokito_sym")),
            &two_pin_passive(name, desc, "SW_*", 1.5, 2.0),
        )?;
        count += 1;
    }

    for (name, desc) in [
        ("1N4148W", "Switching diode SMD"),
        ("SS14", "Schottky SMD"),
        ("BAV99", "Dual diode"),
        ("SMBJ5.0A", "TVS diode"),
        ("MMBT2222", "NPN SMD"),
        ("MMBT2907", "PNP SMD"),
        ("2N7002", "N-MOSFET"),
        ("AO3401", "P-MOSFET"),
        ("IRLZ44N", "Power MOSFET"),
        ("TIP122", "Darlington"),
    ] {
        let kind = if name.starts_with("MM")
            || name.contains("700")
            || name.contains("AO")
            || name.contains("MBT")
            || name.contains("TIP")
        {
            "Q"
        } else {
            "D"
        };
        write_file(
            &out_dir
                .join("Semiconductor")
                .join(format!("{name}.tokito_sym")),
            &semiconductor_3pin(name, desc, kind),
        )?;
        count += 1;
    }

    for (name, pins) in [
        ("MCP6002", 8),
        ("MCP6004", 14),
        ("OPA2134", 8),
        ("AD8605", 5),
        ("LT1013", 8),
        ("MAX232", 16),
        ("CH340G", 16),
        ("W25Q128", 8),
        ("AT24C256", 8),
        ("PCF8574", 16),
    ] {
        write_file(
            &out_dir.join("Interface").join(format!("{name}.tokito_sym")),
            &dual_row_ic(name, pins, name, "U"),
        )?;
        count += 1;
    }

    Ok(count)
}

fn semiconductor_3pin(name: &str, desc: &str, ref_pre: &str) -> String {
    format!(
        r#"{HEADER}	(symbol "{name}"
		(property "Reference" "{ref_pre}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Value" "{name}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Description" "{desc}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(symbol "{name}_0_1"
			(rectangle
				(start -2.0 2.0)
				(end 2.0 -2.0)
				(stroke (width 0.254) (type default))
				(fill (type none))))
		(symbol "{name}_1_1"
			(pin input line (at -5.08 0 0) (length 2.54)
				(number "1" (effects (font (size 1.27 1.27)))))
			(pin passive line (at 0 -5.08 90) (length 2.54)
				(number "2" (effects (font (size 1.27 1.27)))))
			(pin output line (at 5.08 0 180) (length 2.54)
				(number "3" (effects (font (size 1.27 1.27))))))
	)
{FOOTER}"#,
    )
}

fn write_file(path: &Path, body: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        return Ok(());
    }
    fs::write(path, body)?;
    Ok(())
}

fn two_pin_passive(name: &str, desc: &str, fp_filter: &str, half_w: f64, half_h: f64) -> String {
    format!(
        r#"{HEADER}	(symbol "{name}"
		(property "Reference" "R"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Value" "{name}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Footprint" ""
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Description" "{desc}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "fp_filters" "{fp_filter}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(symbol "{name}_0_1"
			(rectangle
				(start -{hw} -{half_h})
				(end {hw} {half_h})
				(stroke (width 0.254) (type default))
				(fill (type none))))
		(symbol "{name}_1_1"
			(pin passive line (at 0 {pin_y} 270) (length 1.27)
				(number "1" (effects (font (size 1.27 1.27)))))
			(pin passive line (at 0 -{pin_y} 90) (length 1.27)
				(number "2" (effects (font (size 1.27 1.27))))))
	)
{FOOTER}"#,
        HEADER = HEADER,
        FOOTER = FOOTER,
        name = name,
        desc = desc,
        fp_filter = fp_filter,
        hw = half_w,
        half_h = half_h,
        pin_y = half_h + 1.27,
    )
}

fn connector_1xn(name: &str, pins: usize) -> String {
    let pitch = 2.54;
    let h = (pins as f64 - 1.0) * pitch;
    let mut body = format!(
        r#"{HEADER}	(symbol "{name}"
		(property "Reference" "J"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Value" "{name}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Footprint" ""
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "fp_filters" "Connector*"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(symbol "{name}_0_1"
			(rectangle
				(start -1.27 1.27)
				(end 1.27 {y_bottom})
				(stroke (width 0.254) (type default))
				(fill (type background))))
		(symbol "{name}_1_1"
"#,
        HEADER = HEADER,
        name = name,
        y_bottom = -h - 1.27,
    );
    for i in 0..pins {
        let y = -(i as f64) * pitch;
        body.push_str(&format!(
            r#"			(pin passive line (at -5.08 {y} 0) (length 3.81)
				(number "{n}" (effects (font (size 1.27 1.27)))))
"#,
            y = y,
            n = i + 1,
        ));
    }
    body.push_str("\t)\n");
    body.push_str(FOOTER);
    body
}

fn dual_row_ic(name: &str, pins: usize, desc: &str, ref_pre: &str) -> String {
    let per_side = pins.div_ceil(2);
    let pitch = 1.27;
    let h = (per_side as f64 - 1.0) * pitch;
    let mut body = format!(
        r#"{HEADER}	(symbol "{name}"
		(property "Reference" "{ref_pre}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Value" "{name}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Description" "{desc}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(symbol "{name}_0_1"
			(rectangle
				(start -5.08 {top})
				(end 5.08 {bottom})
				(stroke (width 0.254) (type default))
				(fill (type background))))
		(symbol "{name}_1_1"
"#,
        HEADER = HEADER,
        name = name,
        ref_pre = ref_pre,
        desc = desc,
        top = h / 2.0 + 1.27,
        bottom = -h / 2.0 - 1.27,
    );
    for i in 0..per_side {
        let y = h / 2.0 - i as f64 * pitch;
        let n_left = i + 1;
        let n_right = i + 1 + per_side;
        if n_left <= pins {
            body.push_str(&format!(
                r#"			(pin input line (at -7.62 {y} 0) (length 2.54)
				(number "{n_left}" (effects (font (size 1.27 1.27)))))
"#,
            ));
        }
        if n_right <= pins {
            body.push_str(&format!(
                r#"			(pin input line (at 7.62 {y} 180) (length 2.54)
				(number "{n_right}" (effects (font (size 1.27 1.27)))))
"#,
            ));
        }
    }
    body.push_str("\t\t)\n\t)\n");
    body.push_str(FOOTER);
    body
}

fn regulator_3pin(name: &str, desc: &str, fp: &str) -> String {
    format!(
        r#"{HEADER}	(symbol "{name}"
		(property "Reference" "U"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Value" "{name}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Description" "{desc}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(property "Footprint" "{fp}"
			(at 0 0 0)
			(effects (font (size 1.27 1.27))))
		(symbol "{name}_0_1"
			(rectangle
				(start -2.54 2.54)
				(end 2.54 -2.54)
				(stroke (width 0.254) (type default))
				(fill (type background))))
		(symbol "{name}_1_1"
			(pin input line (at -5.08 0 0) (length 2.54)
				(number "1" (effects (font (size 1.27 1.27)))))
			(pin power_in line (at 0 5.08 270) (length 2.54)
				(number "2" (effects (font (size 1.27 1.27)))))
			(pin output line (at 5.08 0 180) (length 2.54)
				(number "3" (effects (font (size 1.27 1.27))))))
	)
{FOOTER}"#,
    )
}
