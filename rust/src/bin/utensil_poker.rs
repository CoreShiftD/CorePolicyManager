fn main() {
    if let Err(err) = corepolicy_manager_wrapper::utensil::run() {
        coreshift_core::alog_error!("utensil-poker", "{err}");
        std::process::exit(1);
    }
}
