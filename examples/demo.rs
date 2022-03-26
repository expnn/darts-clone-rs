
use darts_clone::Datrie;


fn main() {
    let mut da = Datrie::new();

    let id = da.find("hello", None);
    println!("{}", id.unwrap_or(-1));
    da.build(&["hello", "world", "he", "hell"], Some(&[0, 1, 2, 3])).expect("build failed");
    let id = da.find("hello", None);
    println!("{}", id.unwrap_or(-1));

    let res = da.common_prefix_search("hello", 10, None);
    println!("{:?}", res);

    da.dump("./target/tmp.bin", None, None).unwrap();
    let mut new_da = Datrie::new();

    new_da.load("./target/tmp.bin", None, None).unwrap();
    let res = new_da.common_prefix_search("hello", 10, None);
    println!("{:?}", res);
}
