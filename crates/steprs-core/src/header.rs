use serde::{Deserialize, Serialize};

/// Parsed HEADER section metadata (Phase 0 introspection).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepHeader {
    pub description: Vec<String>,
    pub file_name: FileNameMeta,
    pub schemas: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileNameMeta {
    pub name: String,
    pub time_stamp: String,
    pub author: Vec<String>,
    pub organization: Vec<String>,
    pub preprocessor_version: String,
    pub originating_system: String,
    pub authorization: String,
}

impl StepHeader {
    /// Best-effort AP detection from FILE_SCHEMA strings.
    pub fn application_protocol(&self) -> ApplicationProtocol {
        let joined = self.schemas.join(" ").to_ascii_uppercase();
        // Schema numbers are authoritative (214 vs 242 share "AUTOMOTIVE_DESIGN" labels).
        if joined.contains("10303 242") || joined.contains("AP242") || joined.contains("AP 242") {
            ApplicationProtocol::Ap242
        } else if joined.contains("10303 214")
            || joined.contains("AP214")
            || joined.contains("AP 214")
            || joined.contains("AUTOMOTIVE_DESIGN_CORE")
            || joined.contains("AUTOMOTIVE_DESIGN")
        {
            ApplicationProtocol::Ap214
        } else if joined.contains("MANAGED_MODEL_BASED_3D_ENGINEERING") || joined.contains("MBD") {
            ApplicationProtocol::Ap242
        } else if joined.contains("AP203")
            || joined.contains("AP 203")
            || joined.contains("CONFIG_CONTROL_DESIGN")
            || joined.contains("CC_DESIGN")
        {
            ApplicationProtocol::Ap203
        } else {
            ApplicationProtocol::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApplicationProtocol {
    Ap203,
    Ap214,
    Ap242,
    Unknown,
}

impl ApplicationProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ap203 => "AP203",
            Self::Ap214 => "AP214",
            Self::Ap242 => "AP242",
            Self::Unknown => "UNKNOWN",
        }
    }
}

pub fn parse_header_section(input: &str) -> Option<StepHeader> {
    let start = input.find("HEADER;")?;
    let end = input[start..].find("ENDSEC;")? + start;
    let block = &input[start..end];

    let mut header = StepHeader::default();

    if let Some(desc) = extract_parameter_list(block, "FILE_DESCRIPTION") {
        header.description = desc
            .into_iter()
            .filter_map(|p| p.as_string().map(str::to_owned))
            .collect();
    }

    if let Some(name_params) = extract_parameter_list(block, "FILE_NAME") {
        header.file_name = FileNameMeta {
            name: name_params
                .first()
                .and_then(|p| p.as_string())
                .unwrap_or("")
                .to_owned(),
            time_stamp: name_params
                .get(1)
                .and_then(|p| p.as_string())
                .unwrap_or("")
                .to_owned(),
            author: name_params
                .get(2)
                .and_then(|p| p.as_list())
                .map(|l| {
                    l.iter()
                        .filter_map(|x| x.as_string().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            organization: name_params
                .get(3)
                .and_then(|p| p.as_list())
                .map(|l| {
                    l.iter()
                        .filter_map(|x| x.as_string().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            preprocessor_version: name_params
                .get(4)
                .and_then(|p| p.as_string())
                .unwrap_or("")
                .to_owned(),
            originating_system: name_params
                .get(5)
                .and_then(|p| p.as_string())
                .unwrap_or("")
                .to_owned(),
            authorization: name_params
                .get(6)
                .and_then(|p| p.as_string())
                .unwrap_or("")
                .to_owned(),
        };
    }

    if let Some(schema_params) = extract_parameter_list(block, "FILE_SCHEMA") {
        header.schemas = schema_params
            .into_iter()
            .filter_map(|p| match p {
                crate::parameter::Parameter::String(s) => Some(s),
                crate::parameter::Parameter::List(items) => Some(
                    items
                        .iter()
                        .filter_map(|x| x.as_string().map(str::to_owned))
                        .collect::<Vec<_>>()
                        .join(" "),
                ),
                _ => p.as_string().map(str::to_owned),
            })
            .collect();
    }

    Some(header)
}

fn extract_parameter_list(
    block: &str,
    keyword: &str,
) -> Option<smallvec::SmallVec<[crate::parameter::Parameter; 8]>> {
    let needle = format!("{keyword}(");
    let start = block.find(&needle)? + needle.len();
    let rest = &block[start..];
    let (remaining, params) = crate::parser::parameter::parse_parameter_list(rest).ok()?;
    let _ = remaining;
    Some(params)
}
