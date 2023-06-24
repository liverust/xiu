use rand::Rng;

macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

pub(crate) use scanf;

pub fn gen_random_string(size: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..size).map(|_| rng.gen_range(0, 9).to_string()).collect()
}
#[cfg(test)]
mod tests {

    #[test]
    fn test_scanf() {
        let str_a = "18:23:08";

        if let (Some(a), Some(b), Some(c), _) =
            scanf!(str_a, |c| c == ':' || c == '.', i64, i64, i64, i64)
        {
            println!("a:{} b:{} c:{} ", a, b, c);
        }
    }
    use super::gen_random_string;

    #[test]
    fn test_gen_random_string() {
        println!("a:{}", gen_random_string(10));
    }
}
