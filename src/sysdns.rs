use std::net::Ipv4Addr;
use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub fn active_alias() -> Option<String> {
    let script = "$p = Get-NetConnectionProfile -ErrorAction SilentlyContinue | Where-Object { $_.IPv4Connectivity -ne 'Disconnected' } | Select-Object -First 1; \
                  if (-not $p) { $p = Get-NetAdapter -ErrorAction SilentlyContinue | Where-Object { $_.Status -eq 'Up' } | Select-Object -First 1 }; \
                  if ($p) { $p.InterfaceAlias }";
    let alias = powershell(script)?;
    let alias = alias.lines().next()?.trim().to_owned();
    if alias.is_empty() {
        None
    } else {
        Some(alias)
    }
}

pub fn current_servers(alias: &str) -> Vec<String> {
    let script = format!(
        "(Get-DnsClientServerAddress -InterfaceAlias '{}' -AddressFamily IPv4 -ErrorAction SilentlyContinue).ServerAddresses -join ','",
        escape(alias)
    );
    match powershell(&script) {
        Some(output) => output
            .split(',')
            .map(|item| item.trim().to_owned())
            .filter(|item| !item.is_empty())
            .collect(),
        None => Vec::new(),
    }
}

pub fn apply(alias: &str, servers: &[Ipv4Addr]) -> Result<(), String> {
    let list = servers
        .iter()
        .map(|ip| format!("'{}'", ip))
        .collect::<Vec<_>>()
        .join(",");
    let script = format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{}' -ServerAddresses @({})",
        escape(alias),
        list
    );
    run_elevated(&script)
}

pub fn reset(alias: &str) -> Result<(), String> {
    let script = format!(
        "Set-DnsClientServerAddress -InterfaceAlias '{}' -ResetServerAddresses",
        escape(alias)
    );
    run_elevated(&script)
}

pub fn apply_and_verify(alias: &str, servers: &[Ipv4Addr], expected: &str) -> Result<(), String> {
    apply(alias, servers)?;
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if current_servers(alias).iter().any(|item| item == expected) {
            return Ok(());
        }
    }
    Err("Изменение DNS не подтверждено".to_owned())
}

pub fn reset_and_verify(alias: &str) -> Result<(), String> {
    reset(alias)?;
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if current_servers(alias).is_empty() {
            return Ok(());
        }
    }
    Err("Не удалось вернуть автоматический DNS".to_owned())
}

fn powershell(script: &str) -> Option<String> {
    let output = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "[Console]::OutputEncoding=[Text.Encoding]::UTF8; {}",
                script
            ),
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn escape(value: &str) -> String {
    value.replace('\'', "''")
}

fn run_elevated(script: &str) -> Result<(), String> {
    let parameters = format!(
        "-NoProfile -NonInteractive -ExecutionPolicy Bypass -WindowStyle Hidden -Command \"{}\"",
        script.replace('"', "`\"")
    );
    shell_execute("powershell.exe", &parameters)
}

fn shell_execute(file: &str, parameters: &str) -> Result<(), String> {
    let verb = wide("runas");
    let file = wide(file);
    let parameters = wide(parameters);

    let result = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE,
        )
    };

    let code = result as isize;
    if code > 32 {
        Ok(())
    } else if code == 5 {
        Err("Запрос прав администратора отклонён".to_owned())
    } else {
        Err(format!("Ошибка запуска изменения DNS: код {}", code))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
