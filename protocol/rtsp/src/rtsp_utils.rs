pub fn time_2_epoch_seconds(time_str: &str) -> i64 {
    let format_string = "%Y%m%dT%H%M%SZ";

    let datetime = match chrono::DateTime::parse_from_str(time_str, format_string) {
        Ok(dt) => dt,
        Err(err) => {
            log::error!("time_str_2_epoch_seconds error: {}", err);
            return -1;
        }
    };

    datetime.timestamp()
}

macro_rules! scanf {
    ( $string:expr, $sep:expr, $( $x:ty ),+ ) => {{
        let mut iter = $string.split($sep);
        ($(iter.next().and_then(|word| word.parse::<$x>().ok()),)*)
    }}
}

pub(crate) use scanf;

#[cfg(test)]
mod tests {

    #[test]
    fn test_scanf() {
        let str_a = "18:23:08";

        // let (a, b, c, d) = scanf!(str_a, |c| c == ':' || c == '.', usize, usize, usize, usize);
        // println!(
        //     "a:{} b:{} c:{} d:{}",
        //     a.unwrap(),
        //     b.unwrap(),
        //     c.unwrap(),
        //     d.unwrap()
        // );

        if let (Some(a), Some(b), Some(c), d) =
            scanf!(str_a, |c| c == ':' || c == '.', i64, i64, i64, i64)
        {
            println!("a:{} b:{} c:{} ", a, b, c);
        }
    }
}
