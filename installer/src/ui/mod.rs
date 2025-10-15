use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use parking_lot::RwLock;
use slint::{ModelRc, SharedString, VecModel};
use tracing::{error, info, warn};

use crate::backend::{partition, Backend};
use crate::backend::tasks::InstallPlan;
use crate::state::{DiskIdentifier, InstallerState};

slint::include_modules!();

pub struct App {
    window: AppWindow,
    backend: Backend,
    state: Arc<RwLock<InstallerState>>,
    _plan: Arc<RwLock<Option<InstallPlan>>>,
}

impl App {
    pub fn new(state: Arc<RwLock<InstallerState>>, backend: Backend) -> Result<Self> {
        let window = AppWindow::new()?;
        window.set_installing(false);
        let plan_holder: Arc<RwLock<Option<InstallPlan>>> = Arc::new(RwLock::new(None));

        let steps = init_steps(&window, &state);

        window.set_install_plan_summary(SharedString::new());
        window.set_install_log(SharedString::new());

        match backend.list_disks() {
            Ok(disks) => {
                info!(count = disks.len(), "discovered block devices");
                apply_disk_inventory(&window, &state, disks);
            }
            Err(err) => {
                warn!("failed to probe block devices: {:#}", err);
                window.set_disk_items(ModelRc::<DiskItem>::default());
            }
        }
        let next_state = state.clone();
        let steps_for_next = Arc::clone(&steps);
        let backend_for_install = backend.clone();
        let plan_store = Arc::clone(&plan_holder);
        let next_weak = window.as_weak();
        window.on_request_next(move || {
            if let Some(window) = next_weak.upgrade() {
                let idx = window.get_current_step_index();
                let total = window.get_total_steps();
                if idx + 1 < total {
                    let new_idx = idx + 1;
                    window.set_current_step_index(new_idx);
                    next_state.write().current_step = new_idx as usize;
                    update_current_step_labels(&window, &steps_for_next, new_idx as usize);
                    window.set_install_log(SharedString::new());
                } else {
                    let plan = match plan_store.read().clone() {
                        Some(plan) => plan,
                        None => match backend_for_install.begin_installation() {
                            Ok(plan) => {
                                info!(step_count = plan.steps().len(), "prepared installation plan");
                                plan_store.write().replace(plan.clone());
                                window.set_install_plan_summary(build_install_plan_summary(&plan));
                                window.set_install_log(SharedString::new());
                                plan
                            }
                            Err(err) => {
                                error!("failed to prepare installation plan: {:#}", err);
                                window.set_install_log(SharedString::from(format!("Plan generation failed: {err:#}")));
                                return;
                            }
                        },
                    };

                    window.set_installing(true);
                    window.set_install_log(SharedString::from("Running installation..."));
                    let window_for_log = window.as_weak();
                    let backend_runner = backend_for_install.clone();
                    std::thread::spawn(move || {
                        let run_result = backend_runner.execute_plan_stream(plan.clone(), |line| {
                            let window_for_log = window_for_log.clone();
                            let line_owned = line.clone();
                            let _ = slint::invoke_from_event_loop(move || {
                                if let Some(window) = window_for_log.upgrade() {
                                    append_log(&window, &line_owned);
                                }
                            });
                        });

                        let final_message = match run_result {
                            Ok(true) => "Installation completed successfully".to_string(),
                            Ok(false) => "Installation stopped due to errors".to_string(),
                            Err(err) => format!("Execution failed: {err:#}"),
                        };

                        let window_for_log = window_for_log.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(window) = window_for_log.upgrade() {
                                append_log(&window, &final_message);
                                window.set_installing(false);
                            }
                        });
                    });
                }
            }
        });

        let back_state = state.clone();
        let steps_for_back = Arc::clone(&steps);
        let back_weak = window.as_weak();
        window.on_request_back(move || {
            if let Some(window) = back_weak.upgrade() {
                let idx = window.get_current_step_index();
                if idx > 0 {
                    let new_idx = idx - 1;
                    window.set_current_step_index(new_idx);
                    back_state.write().current_step = new_idx as usize;
                    update_current_step_labels(&window, &steps_for_back, new_idx as usize);
                }
            }
        });

        let cancel_weak = window.as_weak();
        window.on_request_cancel(move || {
            if let Some(window) = cancel_weak.upgrade() {
                info!(step = window.get_current_step_index(), "cancel pressed");
            }
        });

        let select_state = state.clone();
        let select_plan = Arc::clone(&plan_holder);
        let select_weak = window.as_weak();
        window.on_select_disk(move |index| {
            if let Some(window) = select_weak.upgrade() {
                handle_disk_selection(&window, &select_state, &select_plan, index as usize);
            }
        });

        Ok(Self {
            window,
            backend,
            state,
            _plan: plan_holder,
        })
    }

    pub fn run(self) -> Result<()> {
        let _ = self.backend;
        self.window.run()?;
        Ok(())
    }
}

fn init_steps(window: &AppWindow, state: &Arc<RwLock<InstallerState>>) -> Arc<Vec<StepData>> {
    let steps_vec = default_steps();
    let steps = Arc::new(steps_vec.clone());
    let model: ModelRc<StepData> = Rc::new(VecModel::from(steps_vec)).into();
    window.set_steps(model);
    let count = steps.len();
    window.set_total_steps(count as i32);

    let mut current = state.read().current_step;
    if current >= count && count > 0 {
        current = count - 1;
    }
    state.write().current_step = current;
    window.set_current_step_index(current as i32);

    update_current_step_labels(window, &steps, current);

    steps
}

fn update_current_step_labels(window: &AppWindow, steps: &Arc<Vec<StepData>>, index: usize) {
    if let Some(step) = steps.get(index) {
        window.set_current_step_title(step.title.clone());
        window.set_current_step_subtitle(step.subtitle.clone());
    } else {
        window.set_current_step_title(SharedString::new());
        window.set_current_step_subtitle(SharedString::new());
    }
}

fn apply_disk_inventory(
    window: &AppWindow,
    state: &Arc<RwLock<InstallerState>>,
    disks: Vec<DiskIdentifier>,
) {
    let selected_path = {
        let mut guard = state.write();
        guard.discovered_disks = disks.clone();
        guard.selected_disk.as_ref().map(|disk| disk.path.clone())
    };

    let items_vec: Vec<DiskItem> = disks
        .into_iter()
        .map(|disk| disk_to_item(disk, selected_path.as_deref()))
        .collect();
    let model: ModelRc<DiskItem> = Rc::new(VecModel::from(items_vec)).into();
    window.set_disk_items(model);

    let summary = {
        let guard = state.read();
        build_disk_summary(&guard)
    };
    window.set_disk_selection_summary(summary);
    window.set_install_plan_summary(SharedString::new());
    window.set_install_log(SharedString::new());
}

fn disk_to_item(disk: DiskIdentifier, selected_path: Option<&str>) -> DiskItem {
    let label = disk
        .label
        .as_ref()
        .map(|l| format!("{l} ({})", disk.path))
        .unwrap_or_else(|| disk.path.clone());

    let size = format_size(disk.size_bytes);

    let selected = selected_path
        .map(|path| path == disk.path)
        .unwrap_or(false);

    DiskItem {
        label: label.into(),
        path: disk.path.into(),
        size,
        selected,
    }
}

fn format_size(bytes: u64) -> SharedString {
    const THRESHOLD: f64 = 1024.0;
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];

    if bytes == 0 {
        return SharedString::from("0 B");
    }

    let mut value = bytes as f64;
    let mut idx = 0;

    while value >= THRESHOLD && idx < UNITS.len() - 1 {
        value /= THRESHOLD;
        idx += 1;
    }

    SharedString::from(format!("{value:.1} {}", UNITS[idx]))
}

fn handle_disk_selection(
    window: &AppWindow,
    state: &Arc<RwLock<InstallerState>>,
    plan_store: &Arc<RwLock<Option<InstallPlan>>>,
    index: usize,
) {
    let disks = {
        let mut guard = state.write();
        if let Some(selected) = guard.discovered_disks.get(index).cloned() {
            guard.selected_disk = Some(selected.clone());
            guard.target = Some(partition::default_plan_for_disk(&selected));
        } else {
            warn!(index, count = guard.discovered_disks.len(), "disk selection index out of range");
            return;
        }
        guard.discovered_disks.clone()
    };

    plan_store.write().take();
    window.set_install_plan_summary(SharedString::new());
    window.set_install_log(SharedString::new());

    apply_disk_inventory(window, state, disks);
}

fn append_log(window: &AppWindow, line: &str) {
    let mut current = window.get_install_log().to_string();
    if !current.is_empty() {
        current.push('\n');
    }
    current.push_str(line);
    window.set_install_log(current.into());
}

fn build_disk_summary(state: &InstallerState) -> SharedString {
    if let Some(plan) = state.target.as_ref() {
        let mut lines = Vec::new();
        lines.push(format!(
            "Target disk: {} ({})",
            plan.target.path,
            human_readable_bytes(plan.target.size_bytes)
        ));
        for partition in &plan.partitions {
            lines.push(format!("  - {}", partition::describe_partition(partition)));
        }
        SharedString::from(lines.join("\n"))
    } else if let Some(disk) = state.selected_disk.as_ref() {
        SharedString::from(format!(
            "Selected disk: {} ({})",
            disk.path,
            human_readable_bytes(disk.size_bytes)
        ))
    } else {
        SharedString::from("No disk selected")
    }
}

fn human_readable_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    if bytes == 0 {
        return "0 B".into();
    }

    let mut value = bytes as f64;
    let mut idx = 0;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }

    format!("{value:.1} {}", UNITS[idx])
}

fn build_install_plan_summary(plan: &InstallPlan) -> SharedString {
    let mut lines = Vec::new();
    lines.push("Installation plan:".to_string());
    for (idx, step) in plan.steps().iter().enumerate() {
        lines.push(format!(
            "{}. {:?}: {} ({} commands)",
            idx + 1,
            step.stage,
            step.summary,
            step.commands.len()
        ));
    }

    SharedString::from(lines.join("\n"))
}

fn default_steps() -> Vec<StepData> {
    WIZARD_STEPS
        .iter()
        .map(|(title, subtitle)| StepData {
            title: (*title).into(),
            subtitle: (*subtitle).into(),
        })
        .collect()
}

const WIZARD_STEPS: &[(&str, &str)] = &[
    ("Welcome", "Overview and prerequisites"),
    ("Locale", "Select language and formats"),
    ("Keyboard", "Choose keyboard layout"),
    ("Timezone", "Select region and timezone"),
    ("Disks", "Select target disk and layout"),
    ("Filesystem", "Confirm partition formatting"),
    ("Install", "Install base system"),
    ("Configure", "Users and system settings"),
    ("Finish", "Review and reboot"),
];
