use timely::dataflow::*;
use timely::dataflow::operators::*;
use timely::dataflow::operators::probe::Handle as ProbeHandle;

use differential_dataflow::AsCollection;
use differential_dataflow::operators::*;
use differential_dataflow::lattice::Lattice;

use ::Collections;
use ::types::create_date;

// -- $ID$
// -- TPC-H/TPC-R Potential Part Promotion Query (Q20)
// -- Function Query Definition
// -- Approved February 1998
// :x
// :o
// select
//     s_name,
//     s_address
// from
//     supplier,
//     nation
// where
//     s_suppkey in (
//         select
//             ps_suppkey
//         from
//             partsupp
//         where
//             ps_partkey in (
//                 select
//                     p_partkey
//                 from
//                     part
//                 where
//                     p_name like ':1%'
//             )
//             and ps_availqty > (
//                 select
//                     0.5 * sum(l_quantity)
//                 from
//                     lineitem
//                 where
//                     l_partkey = ps_partkey
//                     and l_suppkey = ps_suppkey
//                     and l_shipdate >= date ':2'
//                     and l_shipdate < date ':2' + interval '1' year
//             )
//     )
//     and s_nationkey = n_nationkey
//     and n_name = ':3'
// order by
//     s_name;
// :n -1

fn starts_with(source: &[u8], query: &[u8]) -> bool {
    source.len() >= query.len() && &source[..query.len()] == query
}

pub fn query<G: Scope>(collections: &mut Collections<G>) -> ProbeHandle<G::Timestamp> 
where G::Timestamp: Lattice+Ord {

    let partkeys = collections.parts.filter(|p| p.name.as_bytes() == b"forest").map(|p| p.part_key);

    let available = 
    collections
        .lineitems()
        .flat_map(|l| 
            if l.ship_date >= create_date(1994, 1, 1) && l.ship_date < create_date(1995, 1, 1) { Some((l.part_key, (l.supp_key, l.quantity))) }
            else { None }
        )
        .semijoin_u(&partkeys)
        .inner
        .map(|(l, t, d)| (((l.0 as u64) << 32) + (l.1).0 as u64, t, (l.1).1 as isize * d))
        .as_collection()
        .count();

    let suppliers = 
    collections
        .partsupps()
        .map(|ps| (ps.part_key, (ps.supp_key, ps.availqty)))
        .semijoin_u(&partkeys)
        .map(|(part_key, (supp_key, avail))| (((part_key as u64) << 32) + (supp_key as u64), avail))
        .join_u(&available)   // TODO: these could be fused into a u64, for join_u.
        .filter(|&(_, avail1, avail2)| avail1 > avail2 as i32 / 2)
        .map(|(part_supp_fuse, _, _)| (part_supp_fuse & (u32::max_value() as u64)) as usize);

    let nations = collections.nations.filter(|n| starts_with(&n.name, b"CANADA")).map(|n| (n.nation_key, n.name));

    collections
        .suppliers()
        .map(|s| (s.supp_key, (s.name, s.address.to_string(), s.nation_key)))
        .semijoin_u(&suppliers)
        .map(|(_, (name, addr, nation))| (nation, (name, addr)))
        .join_u(&nations)
        .probe()
}