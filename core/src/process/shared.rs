use std::env;

pub fn restart_as_root() {
    //println!("{}", env::current_exe().unwrap().display());
    //println!("{}", env::args().collect::<Vec<String>>().join(" "));
    let args = env::args().collect::<Vec<String>>();
    let mut cmd = runas::Command::new(&env::current_exe().unwrap());
    cmd.gui(true);    
    cmd.force_prompt(true);
    cmd.args(&args[1..]);
    println!("{}",cmd.status().expect("failed to execute"));
    std::process::exit(0);
}
