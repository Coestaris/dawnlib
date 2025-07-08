use log::info;

pub fn profile_threads(frame: &yage2_core::threads::ProfileFrame) {
    let mut str = String::with_capacity(1024);
    for thread in &frame.threads {
        str.push_str(&format!(
            "{}: ({:.1}) ",
            thread.name,
            thread.cpu_utilization * 100.0
        ));
    }
    info!("{}", str);
}
