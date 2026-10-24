use std::collections::HashMap;

use slot_gfx::ShaderParam;
use slot_input::{Action, Btn};
use slot_ui::{ListPage, ListRow};

use super::overlay::Overlay;
use super::App;

#[derive(Default)]
pub struct ShaderSettingsState {
    pub shader_name: Option<String>,
    pub params: Vec<ShaderParam>,
    pub values: HashMap<String, f32>,
    pub pending: Vec<(String, f32)>,
}

fn format_value(val: f32, step: f32) -> String {
    if (step - step.round()).abs() < 1e-4 && (val - val.round()).abs() < 1e-4 {
        format!("{:.0}", val)
    } else if ((step * 10.0) - (step * 10.0).round()).abs() < 1e-4 {
        format!("{:.1}", val)
    } else {
        format!("{:.2}", val)
    }
}

impl ShaderSettingsState {
    pub fn rows(&self) -> Vec<ListRow> {
        let mut rows = Vec::with_capacity(self.params.len() + 1);
        for p in &self.params {
            let val = self.values.get(&p.id).copied().unwrap_or(p.default);
            let val_str = format_value(val, p.step);
            rows.push(ListRow::new(p.label.as_str()).with_value(val_str));
        }
        rows.push(ListRow::new("Reset"));
        rows
    }
}

impl App {
    pub(super) fn open_shader_settings(&mut self) {
        let Some(name) = self.card_shader() else {
            return;
        };
        let Some(root) = self.root().map(|p| p.to_path_buf()) else {
            return;
        };
        let Some(path) = slot_store::shader_path(&root, &name) else {
            return;
        };
        let Ok(src) = std::fs::read_to_string(path) else {
            return;
        };
        let params = slot_gfx::parse_parameters(&src);
        if params.is_empty() {
            return;
        }

        let saved = slot_store::load_shader_params(&root, &name);
        let mut values = HashMap::new();
        for p in &params {
            let val = saved.get(&p.id).copied().unwrap_or(p.default);
            values.insert(p.id.clone(), val);
        }

        let st = &mut self.overlay.shader_settings;
        st.shader_name = Some(name);
        st.params = params;
        st.values = values;

        let rows = st.rows();
        self.overlay.open = Some(Overlay::ShaderSettings(ListPage::new(rows)));
        self.overlay.faces.clear();
    }

    pub(super) fn shader_settings_input(&mut self, action: Action) -> bool {
        let Some(Overlay::ShaderSettings(page)) = &mut self.overlay.open else {
            return false;
        };
        let at = page.cursor();
        let root = self.root().map(|p| p.to_path_buf());
        let st = &mut self.overlay.shader_settings;
        let num_params = st.params.len();

        if at < num_params {
            let right = match action {
                Action::GbaDown(Btn::Right) => true,
                Action::GbaDown(Btn::Left) => false,
                _ => return false,
            };
            let p = &st.params[at];
            let cur = st.values.get(&p.id).copied().unwrap_or(p.default);
            let next = if right {
                (cur + p.step).min(p.max)
            } else {
                (cur - p.step).max(p.min)
            };
            let snapped = ((next / p.step).round() * p.step).clamp(p.min, p.max);
            st.values.insert(p.id.clone(), snapped);
            st.pending.push((p.id.clone(), snapped));

            let rows = st.rows();
            if let Some(Overlay::ShaderSettings(page)) = &mut self.overlay.open {
                page.set_rows(rows);
            }

            if let (Some(root), Some(name)) = (&root, &st.shader_name) {
                let _ = slot_store::save_shader_params(root, name, &st.values);
            }
            return true;
        }

        if at == num_params {
            if action != Action::GbaDown(Btn::A) {
                return false;
            }
            for p in &st.params {
                st.values.insert(p.id.clone(), p.default);
                st.pending.push((p.id.clone(), p.default));
            }

            let rows = st.rows();
            if let Some(Overlay::ShaderSettings(page)) = &mut self.overlay.open {
                page.set_rows(rows);
            }

            if let (Some(root), Some(name)) = (&root, &st.shader_name) {
                let _ = slot_store::save_shader_params(root, name, &st.values);
            }
            return true;
        }

        false
    }

    pub fn drain_shader_param_changes(&mut self) -> Vec<(String, f32)> {
        std::mem::take(&mut self.overlay.shader_settings.pending)
    }

    pub fn current_shader_has_params(&self) -> bool {
        let Some(name) = self.card_shader() else {
            return false;
        };
        let Some(root) = self.root() else {
            return false;
        };
        let Some(path) = slot_store::shader_path(root, &name) else {
            return false;
        };
        let Ok(src) = std::fs::read_to_string(path) else {
            return false;
        };
        !slot_gfx::parse_parameters(&src).is_empty()
    }
}
