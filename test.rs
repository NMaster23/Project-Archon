use rand::RngCore;
fn test() {
    let mut new_key = [0u8; 32];
    rand::rng().fill_bytes(&mut new_key);
}
