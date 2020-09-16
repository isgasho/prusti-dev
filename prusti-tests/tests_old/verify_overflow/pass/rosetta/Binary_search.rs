//! An adaptation of the example from
//! https://rosettacode.org/wiki/Binary_search#Rust
//!
//! Changes:
//!
//! +   Monomorphised types.
//! +   Wrapped built-in types and functions.
//! +   Replaced a slice with a reference into a vector.
//! +   Rewrote to remove a return statement.
//!
//! Verified properties:
//!
//! +   Absence of panics.
//! +   Absence of overflows.
//! +   If the result is `None`, then the input vector did not contain the
//!     element.
//! +   If the result is `Some(index)` then the `arr[index] == elem`.
//!
//! The original example contains a bug, which can be showed by using
//! the following counter-example:
//!
//! ```rust
//! fn main() {
//!     let v = vec![0, 1, 2, 3, 4, 5, 6];
//!     println!("{:?}", binary_search(&v, &6));
//! }
//! ```
//!
//! This program should print `Some(6)`, but it prints None. The fixed
//! version would be:
//!
//! ```rust
//! use std::cmp::Ordering::*;
//! fn binary_search<T: Ord>(arr: &[T], elem: &T) -> Option<usize>
//! {
//!     let mut size = arr.len();
//!     let mut base = 0;
//!  
//!     while size > 0 {
//!         let half = size / 2;
//!         let mid = base + half;
//!         base = match arr[mid].cmp(elem) {
//!             Less    => mid,
//!             Greater => base,
//!             Equal   => return Some(mid)
//!         };
//!         size -= half;
//!     }
//!  
//!     None
//! }
//! ```
//!
//! This file contains a verified version of it.

#![allow(dead_code)]
use prusti_contracts::*;

pub struct VecWrapperI32{
    v: Vec<i32>
}

impl VecWrapperI32 {
    #[trusted]
    #[pure]
    pub fn len(&self) -> usize {
        self.v.len()
    }

    #[trusted]
    #[ensures(result.len() == 0)]
    pub fn new() -> Self {
        VecWrapperI32{ v: Vec::new() }
    }

    #[trusted]
    #[pure]
    #[requires(0 <= index && index < self.len())]
    pub fn lookup(&self, index: usize) -> i32 {
        self.v[index]
    }

    #[trusted]
    #[requires(0 <= index && index < self.len())]
    #[ensures(*result == old(self.lookup(index)))]
    #[ensures(after_expiry(
        self.len() == old(self.len()) &&
        self.lookup(index) == before_expiry(*result) &&
        forall(|i: usize| (0 <= i && i < self.len() && i != index) ==>
            self.lookup(i) == old(self.lookup(i)))
    ))]
    pub fn borrow(&mut self, index: usize) -> &mut i32 {
        self.v.get_mut(index).unwrap()
    }

    #[trusted]
    #[ensures(self.len() == old(self.len()) + 1)]
    #[ensures(self.lookup(old(self.len())) == value)]
    #[ensures(forall(|i: usize| (0 <= i && i < old(self.len())) ==>
                    self.lookup(i) == old(self.lookup(i))))]
    pub fn push(&mut self, value: i32) {
        self.v.push(value);
    }
}

enum UsizeOption {
    Some(usize),
    None,
}

impl UsizeOption {
    #[pure]
    fn is_some(&self) -> bool {
        match self {
            UsizeOption::Some(_) => true,
            UsizeOption::None => false,
        }
    }
    #[pure]
    fn is_none(&self) -> bool {
        !self.is_some()
    }
    #[pure]
    #[requires(self.is_some())]
    fn peek(&self) -> usize {
        match self {
            UsizeOption::Some(n) => *n,
            UsizeOption::None => unreachable!(),
        }
    }
}

pub enum Ordering {
    Less,
    Equal,
    Greater,
}

use self::Ordering::*;

// Adapted from https://doc.rust-lang.org/src/core/cmp.rs.html#962-966
#[ensures(*a == old(*a))]
#[ensures(*b == old(*b))]
#[ensures((match result {
                Equal => *a == *b,
                Less => *a < *b,
                Greater => *a > *b,
            }))]
fn cmp(a: &mut i32, b: &mut i32) -> Ordering {
    if *a == *b { Equal }
        else if *a < *b { Less }
            else { Greater }
}

#[requires(forall k1: usize, k2: usize :: (0 <= k1 && k1 < k2 && k2 < arr.len()) ==>
             arr.lookup(k1) <= arr.lookup(k2))]
#[ensures(arr.len() == old(arr.len()))]
#[ensures(forall k: usize:: (0 <= k && k < arr.len()) ==> arr.lookup(k) == old(arr.lookup(k)))]
#[ensures(*elem == old(*elem))]
#[ensures(result.is_none() ==>
            (forall k: usize :: (0 <= k && k < arr.len()) ==> *elem != arr.lookup(k)))]
#[ensures(result.is_some() ==> (
                0 <= result.peek() && result.peek() < arr.len() &&
                arr.lookup(result.peek()) == *elem))]
fn binary_search(arr: &mut VecWrapperI32, elem: &mut i32) -> UsizeOption
{
    let mut size = arr.len();
    let mut base = 0;

    let mut result = UsizeOption::None;
    let mut continue_loop = size > 0;

    while continue_loop {
        body_invariant!(base + size <= arr.len());
        body_invariant!(size > 0 && result.is_none());
        body_invariant!(arr.len() == old(arr.len()));
        body_invariant!(*elem == old(*elem));
        body_invariant!(forall k1: usize, k2: usize :: (0 <= k1 && k1 < k2 && k2 < arr.len()) ==>
            arr.lookup(k1) <= arr.lookup(k2));
        body_invariant!(forall k: usize:: (0 <= k && k < arr.len()) ==> arr.lookup(k) == old(arr.lookup(k)));
        body_invariant!(forall k: usize:: (0 <= k && k < base) ==> arr.lookup(k) < *elem);
        body_invariant!(result.is_none() ==>
             (forall k: usize:: (base + size <= k && k < arr.len()) ==> *elem < arr.lookup(k)));
        body_invariant!(result.is_some() ==> (
                0 <= result.peek() && result.peek() < arr.len() &&
                arr.lookup(result.peek()) == *elem));
        let half = size / 2;
        let mid = base + half;

        let mid_element = arr.borrow(mid);
        let cmp_result = cmp(mid_element, elem);
        base = match cmp_result {
            Less    => {
                mid
            },
            Greater => {
                base
            },
            // Equal
            _   => {
                result = UsizeOption::Some(mid);
                base   // Just return anything because we are finished.
            }
        };
        size -= half;
        continue_loop = size > 0 && result.is_none();
    }

    result
}

fn main() {}
