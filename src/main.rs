mod account;
mod assistant;
mod catalog;
mod compare;
mod decode;
mod eelog;
mod goals;
mod icons;
mod insight;
mod market;
mod recipes;
mod recommend;
mod settings;
mod share;
mod trade;
mod util;
mod worldstate;

fn main() {
    let c = catalog::Catalog::load();
    println!("{} primes, {} relics", c.primes.len(), c.relic_count());
}
