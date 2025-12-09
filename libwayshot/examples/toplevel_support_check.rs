use libwayshot::WayshotConnection;
fn main() {
    let wayshot_conn = WayshotConnection::new().unwrap();

    println!(
        "toplevel image copy supported: {}",
        wayshot_conn.toplevel_capture_support()
    );
}
