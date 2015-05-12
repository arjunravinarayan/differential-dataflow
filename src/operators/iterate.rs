use std::num::One;

use timely::example_shared::*;
use timely::example_shared::operators::*;

use timely::communication::*;
use timely::progress::nested::product::Product;
use timely::progress::nested::Summary::Local;
use timely::progress::timestamp::Timestamp;

use columnar::Columnar;

use collection_trace::lookup::UnsignedInt;
use collection_trace::LeastUpperBound;
use operators::ExceptExt;
use operators::ConsolidateExt;

pub trait IterateExt<G: GraphBuilder, D: Data> {
    fn iterate<Q: Fn()->P,
               P: Fn(&D)->U+'static,
               U: UnsignedInt,
               F: FnOnce(&Stream<SubgraphBuilder<G, T>, (D,i32)>)->
                         Stream<SubgraphBuilder<G, T>, (D,i32)>,
               T: Timestamp+LeastUpperBound=u64,
               >
        (&self, iterations: T, part: Q, logic: F) -> Stream<G, (D,i32)> where G::Timestamp: LeastUpperBound, T::Summary: One;
}

impl<G: GraphBuilder, D: Ord+Data+Columnar> IterateExt<G, D> for Stream<G, (D, i32)> {
    fn iterate<Q: Fn()->P,
               P: Fn(&D)->U+'static,
               U: UnsignedInt,
               F: FnOnce(&Stream<SubgraphBuilder<G, T>, (D,i32)>)->
                         Stream<SubgraphBuilder<G, T>, (D,i32)>,
               T: Timestamp+LeastUpperBound=u64,
               >
        (&self, iterations: T, part: Q, logic: F) -> Stream<G, (D,i32)>
where G::Timestamp: LeastUpperBound, T::Summary: One {

        self.builder().subcomputation(|subgraph| {

            let (feedback, cycle) = subgraph.loop_variable(Product::new(G::Timestamp::max(), iterations), Local(T::Summary::one()));
            let ingress = subgraph.enter(&self);

            let bottom = logic(&ingress.concat(&cycle));

            bottom.except(&ingress).consolidate(part).connect_loop(feedback);
            bottom.leave()
        })
    }
}