use super::*;

use std::fmt::{Debug,Formatter,Error};
impl<Item: Debug + Copy + MetricSpace<UserImpl>, UserImpl, Ownership> Debug for Tree<Item, UserImpl, Ownership> {
    fn fmt(&self, f:&mut Formatter) -> Result<(),Error> {
        write!(f, "digraph \"vp tree.dot\" {{\n{:?}}}", self.root)
    }
}

impl<Item: Debug + Copy + MetricSpace<UserImpl>, UserImpl> Debug for Node<Item, UserImpl> {
    fn fmt(&self, f:&mut Formatter) -> Result<(),Error> {
        if self.near.is_some() {
            try!(write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.near.as_ref().unwrap().vantage_point));
            try!(self.near.as_ref().unwrap().fmt(f));
        }
        if self.far.is_some() {
            try!(write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.far.as_ref().unwrap().vantage_point));
            try!(self.far.as_ref().unwrap().fmt(f));
        }
        return Ok(());
    }
}
