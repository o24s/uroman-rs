use crate::core::{UromanInner, Value};
use num_rational::Ratio;
use serde::Serialize;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, Serialize, PartialEq, PartialOrd)]
pub struct EdgeData {
    pub start: usize,
    pub end: usize,
    pub txt: String,
    pub r#type: String,
}

#[derive(Debug, Clone, Default, PartialEq, PartialOrd, Serialize)]
pub struct NumData {
    pub orig_txt: String,
    pub value: Option<f64>,
    pub fraction: Option<Ratio<i64>>,
    pub num_base: Option<i64>,
    pub base_multiplier: Option<f64>,
    pub script: Option<String>,
    pub is_large_power: bool,
    pub active: bool,
    pub value_s: Option<String>,
    pub n_decimals: Option<usize>,
}

/// A dedicated struct for flexibly updating fields of a `NumData`.
/// This mimics Python's keyword arguments, allowing partial updates.
#[derive(Default, Debug)]
pub struct NumDataUpdates {
    pub value: Option<f64>,
    pub fraction: Option<Ratio<i64>>,
    pub num_base: Option<i64>,
    pub base_multiplier: Option<f64>,
    pub r#type: Option<String>,
    pub script: Option<String>,
    pub is_large_power: Option<bool>,
    pub active: Option<bool>,
    pub n_decimals: Option<usize>,
    pub orig_txt: Option<String>,
    pub value_s: Option<String>,
}

/// A unified Edge type.
#[derive(Debug, Clone, Serialize, PartialOrd)]
pub enum Edge {
    Regular(EdgeData),
    Numeric { data: EdgeData, num_data: NumData },
}

impl Hash for Edge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let d = self.get_data();
        d.start.hash(state);
        d.end.hash(state);
        d.txt.hash(state);
        d.r#type.hash(state);
    }
}

impl PartialEq for Edge {
    fn eq(&self, other: &Self) -> bool {
        let d1 = self.get_data();
        let d2 = other.get_data();
        d1.start == d2.start && d1.end == d2.end && d1.txt == d2.txt && d1.r#type == d2.r#type
    }
}

impl Eq for Edge {}

impl Edge {
    /// Creates a regular edge.
    pub fn new_regular(start: usize, end: usize, txt: String, r#type: String) -> Self {
        Edge::Regular(EdgeData {
            start,
            end,
            txt,
            r#type,
        })
    }

    /// Creates an initial numeric edge from `uroman.num_props`.
    pub(crate) fn new_numeric(
        start: usize,
        end: usize,
        char: char,
        uroman: &UromanInner,
    ) -> Option<Self> {
        let props_map = uroman.num_props.get(&char.to_string())?;

        let rom_text = props_map
            .get("rom")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| char.to_string());

        let value = props_map.get("value").and_then(|v| match v {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        });

        let fraction = props_map.get("fraction").and_then(|v| match v {
            Value::String(s) => s
                .split_once('/')
                .and_then(|(num, den)| Some(Ratio::new(num.parse().ok()?, den.parse().ok()?))),
            Value::Array(arr) if arr.len() == 2 => {
                if let (Some(Value::Int(num)), Some(Value::Int(den))) = (arr.first(), arr.get(1)) {
                    if *den != 0 {
                        Some(Ratio::new(*num, *den))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        });

        let r#type = props_map
            .get("type")
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();

        let is_large_power = props_map
            .get("is-large-power")
            .is_some_and(|v| matches!(v, Value::Int(1)));

        let num_base = props_map.get("base").and_then(|v| match v {
            Value::Int(i) => Some(*i),
            _ => None,
        });

        let base_multiplier = props_map.get("mult").and_then(|v| match v {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        });

        let script = props_map.get("script").and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        });

        let edge = Edge::Numeric {
            data: EdgeData {
                start,
                end,
                txt: rom_text,
                r#type,
            },
            num_data: NumData {
                orig_txt: char.to_string(),
                value,
                fraction,
                num_base,
                base_multiplier,
                script,
                is_large_power,
                active: true,
                ..Default::default()
            },
        };
        Some(edge)
    }

    /// Creates a new combined numeric edge from multiple existing edges.
    ///
    /// # Arguments
    /// * `start` - The start position of the new edge.
    /// * `end` - The end position of the new edge.
    /// * `value` - The combined numeric value as an f64.
    /// * `e_type` - The type of the new edge (e.g., "D1", "G1", "G2").
    /// * `script` - The script of the edge (optional).
    /// * `num_base` - The base of the new numeric edge (optional).
    /// * `n_decimals` - The number of decimal places (optional).
    /// * `orig_txt` - The original text (concatenation of orig_txt from previous edges).
    pub fn new_combined_numeric(
        start: usize,
        end: usize,
        value: f64,
        e_type: String,
        script: Option<String>,
        num_base: Option<i64>,
        n_decimals: Option<usize>,
        orig_txt: String, // Accepts the combined original text.
    ) -> Self {
        let num_data = NumData {
            orig_txt,
            value: Some(value),
            num_base,
            script,
            is_large_power: false,
            active: true,
            n_decimals,
            ..Default::default()
        };

        let mut edge = Edge::Numeric {
            data: EdgeData {
                start,
                end,
                txt: "".to_string(),
                r#type: e_type,
            },
            num_data,
        };

        edge.update(NumDataUpdates::default());
        edge
    }

    /// Returns the original text (`orig_txt`) of the edge.
    /// For `Edge::Numeric`, it returns `orig_txt` from `NumData`.
    /// For `Edge::Regular`, it returns an empty string.
    pub fn orig_txt(&self) -> &str {
        match self {
            Edge::Numeric { num_data, .. } => &num_data.orig_txt,
            Edge::Regular(_) => "", // Regular edges don't have orig_txt, so return an empty string.
        }
    }

    pub fn is_large_power(&self) -> bool {
        self.get_num_data().is_some_and(|d| d.is_large_power)
    }

    pub fn get_num_base(&self) -> Option<i64> {
        self.get_num_data().and_then(|d| d.num_base)
    }

    pub fn get_script(&self) -> Option<String> {
        self.get_num_data().and_then(|d| d.script.clone())
    }

    pub fn is_numeric(&self) -> bool {
        matches!(self, Edge::Numeric { .. })
    }

    /// Updates the properties of a numeric edge and recalculates `txt` (the romanized representation) accordingly.
    pub fn update(&mut self, updates: NumDataUpdates) {
        if let Edge::Numeric { num_data, data } = self {
            // --- Update data from the `updates` struct ---
            if let Some(v) = updates.value {
                num_data.value = Some(v);
            }
            if let Some(v) = updates.fraction {
                num_data.fraction = Some(v);
            }
            if let Some(v) = updates.num_base {
                num_data.num_base = Some(v);
            }
            if let Some(v) = updates.base_multiplier {
                num_data.base_multiplier = Some(v);
            }
            if let Some(v) = updates.r#type {
                data.r#type = v;
            } // Update EdgeData's type
            if let Some(v) = updates.script {
                num_data.script = Some(v);
            }
            if let Some(v) = updates.is_large_power {
                num_data.is_large_power = v;
            }
            if let Some(v) = updates.active {
                num_data.active = v;
            }
            if let Some(v) = updates.n_decimals {
                num_data.n_decimals = Some(v);
            }
            if let Some(v) = updates.orig_txt {
                num_data.orig_txt = v;
            }
            if let Some(v) = updates.value_s {
                num_data.value_s = Some(v);
            }

            // --- Recalculate the display text (`txt`) after all updates ---
            self.recalculate_numeric_txt();
        }
    }

    /// Helper function to recalculate the display text for a numeric edge based on its current data.
    /// This should be called after any modification to `NumData`.
    fn recalculate_numeric_txt(&mut self) {
        if let Edge::Numeric { num_data, data } = self {
            // Determine the primary string for the value, prioritizing `value_s`.
            let value_s = if let Some(vs) = &num_data.value_s {
                vs.clone()
            } else if let Some(v) = num_data.value {
                if let Some(nd) = num_data.n_decimals {
                    format!("{v:.nd$}")
                } else if v.fract() == 0.0 {
                    (v as i64).to_string()
                } else {
                    v.to_string()
                }
            } else {
                "".to_string()
            };

            // Format the fraction part.
            let fraction_s = num_data
                .fraction
                .map(|f| format!("{}/{}", f.numer(), f.denom()))
                .unwrap_or_default();

            let delimiter = if !value_s.is_empty() && !fraction_s.is_empty() {
                " "
            } else {
                ""
            };

            let final_txt = format!("{value_s}{delimiter}{fraction_s}");

            // Fallback to original text if the calculated text is empty.
            data.txt = if final_txt.is_empty() {
                num_data.orig_txt.clone()
            } else {
                final_txt
            };
        }
    }

    // --- Accessors for common data ---
    pub fn get_data(&self) -> &EdgeData {
        match self {
            Edge::Regular(data) | Edge::Numeric { data, .. } => data,
        }
    }

    pub fn get_data_mut(&mut self) -> &mut EdgeData {
        match self {
            Edge::Regular(data) | Edge::Numeric { data, .. } => data,
        }
    }

    // --- Accessors for Numeric data (immutable/mutable) ---
    pub fn get_num_data(&self) -> Option<&NumData> {
        match self {
            Edge::Numeric { num_data, .. } => Some(num_data),
            _ => None,
        }
    }

    pub fn get_num_data_mut(&mut self) -> Option<&mut NumData> {
        match self {
            Edge::Numeric { num_data, .. } => Some(num_data),
            _ => None,
        }
    }

    pub fn start(&self) -> usize {
        self.get_data().start
    }
    pub fn end(&self) -> usize {
        self.get_data().end
    }
    pub fn txt(&self) -> &str {
        &self.get_data().txt
    }
    pub fn r#type(&self) -> &str {
        &self.get_data().r#type
    }

    pub fn is_active(&self) -> bool {
        self.get_num_data().is_none_or(|d| d.active)
    }

    pub fn set_active(&mut self, active: bool) {
        if let Some(d) = self.get_num_data_mut() {
            d.active = active;
        }
    }

    pub fn value(&self) -> Option<f64> {
        self.get_num_data().and_then(|d| d.value)
    }
}
