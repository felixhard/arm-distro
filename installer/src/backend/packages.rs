use crate::backend::command::CommandSpec;

const BASE_PACKAGES: &[&str] = &[
    "base",
    "linux-zen",
    "linux-zen-firmware",
    "openssh",
    "networkmanager",
];

const DESKTOP_PACKAGES: &[&str] = &[
    "gdm",
    "gnome-shell",
    "gnome-control-center",
    "gnome-terminal",
    "nautilus",
    "xdg-user-dirs",
    "gnome-text-editor",
];

pub fn pacstrap_command(root: &str, packages: &[&str]) -> CommandSpec {
    let mut args = Vec::with_capacity(2 + packages.len());
    args.push(root.into());
    args.extend(packages.iter().map(|pkg| pkg.to_string()));
    CommandSpec::new("pacstrap", args)
}

pub fn install_base_packages(root: &str) -> CommandSpec {
    pacstrap_command(root, BASE_PACKAGES)
}

pub fn install_desktop_packages(root: &str) -> CommandSpec {
    pacstrap_command(root, DESKTOP_PACKAGES)
}

pub fn enable_services_commands(root: &str) -> Vec<CommandSpec> {
    vec![
        systemctl_enable_command(root, "gdm.service"),
        systemctl_enable_command(root, "NetworkManager.service"),
        systemctl_enable_command(root, "sshd.service"),
    ]
}

fn systemctl_enable_command(root: &str, service: &str) -> CommandSpec {
    CommandSpec::new(
        "arch-chroot",
        vec![
            root.into(),
            "systemctl".into(),
            "enable".into(),
            service.into(),
        ],
    )
}
