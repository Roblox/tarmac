use packos::{InputItem, SimplePacker};

fn main() {
    env_logger::init();

    let inputs: Vec<_> = (0..5).map(|_| InputItem::new((128, 128))).collect();

    let packer = SimplePacker::new().max_size((256, 256));
    let result = packer.pack(inputs);

    println!("Pack result: {:#?}", result);
}
