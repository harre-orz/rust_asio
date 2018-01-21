use core::{AsIoContext, ThreadIoContext};
use handler::{Complete, Handler, NoYield};

use std::marker::PhantomData;
use std::sync::Arc;
