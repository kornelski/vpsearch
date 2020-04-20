use super::*;

use std::fmt::{Debug,Formatter,Error};
impl<Item: Debug + Clone + MetricSpace<UserImpl>, UserImpl, Ownership> Debug for Tree<Item, UserImpl, Ownership> {
    fn fmt(&self, f:&mut Formatter<'_>) -> Result<(),Error> {
        write!(f, "digraph \"vp tree.dot\" {{\n{:?}}}", self.root)
    }
}

impl<Item: Debug + Clone + MetricSpace<UserImpl>, UserImpl> Debug for Node<Item, UserImpl> {
    fn fmt(&self, f:&mut Formatter<'_>) -> Result<(),Error> {
        if self.near != NO_NODE {
            write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.near)?;
        }
        if self.far != NO_NODE {
            write!(f, "\"{:?}\" -> \"{:?}\"\n", self.vantage_point, self.far)?;
        }
        return Ok(());
    }
}
