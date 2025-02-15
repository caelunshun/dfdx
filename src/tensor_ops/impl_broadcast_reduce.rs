use super::utils::move_tape_and_add_backward_op;
use crate::arrays::{AllAxes, Axes2, Axes3, Axis, HasArrayType};
use crate::devices::{AddAccum, CopyAccum, Cpu, DeviceReduce};
use crate::gradients::Tape;
use crate::prelude::*;

/// Broadcast self into `T` along `Axes`. Opposite of [Reduce].
pub trait BroadcastTo<T, Axes> {
    /// Broadcast `self` into `T`. This can be used to broadcast 1, 2, 3, and 4 axes.
    ///
    /// Examples:
    /// ```rust
    /// # use dfdx::prelude::*;
    /// // broadcast axis 1
    /// let _: Tensor3D<3, 5, 7> = Tensor2D::<3, 7>::zeros().broadcast();
    ///
    /// // broadcast axes 0, 1
    /// let _: Tensor3D<7, 5, 3> = Tensor1D::<3>::zeros().broadcast();
    ///
    /// // broadcast axes 1, 2, 3
    /// let _: Tensor4D<3, 5, 7, 9> = Tensor1D::<3>::zeros().broadcast();
    /// ```
    fn broadcast(self) -> T;
}

/// Remove `Axes` of tensor by reducing them. Opposite of [BroadcastTo].
///
/// Enables functions like [sum()] that reduce values along a single dimension.
///
/// This trait can't be used directly as it doesn't contain any methods. Instead
/// it is used by methods to specify the input type must be able to have it's axes
/// reduced.
pub trait Reduce<Axes>: Sized + Tensor<Dtype = f32> {
    /// The resulting tensor type.
    /// This can be broadcast into Self via [BroadcastTo].
    type Reduced: BroadcastTo<Self, Axes> + Tensor<Tape = Self::Tape, Dtype = Self::Dtype>;
    type DeviceR: DeviceReduce<Self::Array, Axes, Reduced = <Self::Reduced as HasArrayType>::Array>;
}

/// Reduce `Axes` of `Self` to produce a `T`
pub trait ReduceTo<T, Axes>: Reduce<Axes, Reduced = T> {}

macro_rules! impl_broadcast_reduce {
    ($SrcTy:ty, $AxesTy:ty, $DstTy:ty, {$($Dims:tt),*}) => {
impl<$(const $Dims: usize, )* H: Tape> Reduce<$AxesTy> for $DstTy {
    type Reduced = $SrcTy;
    type DeviceR = <Self as HasDevice>::Device;
}

impl<$(const $Dims: usize, )* H: Tape> ReduceTo<$SrcTy, $AxesTy> for $DstTy {}

impl<$(const $Dims: usize, )* H: Tape> BroadcastTo<$DstTy, $AxesTy> for $SrcTy {
    fn broadcast(self) -> $DstTy {
        let mut result = <$DstTy as Tensor>::NoTape::zeros();
        <Cpu as DeviceReduce<_, $AxesTy>>::broadcast_into::<CopyAccum>(result.mut_data(), self.data());
        move_tape_and_add_backward_op(self, result, move |t, result, grads| {
            let (t_grad, result_grad) = grads.mut_and_ref(&t, &result);
            <Cpu as DeviceReduce<_, $AxesTy>>::reduce_into_no_reset::<AddAccum>(t_grad, result_grad);
        })
    }
}
    };
}

impl<H: Tape> Reduce<AllAxes> for Tensor0D<H> {
    type Reduced = Self;
    type DeviceR = <Self as HasDevice>::Device;
}
impl<H: Tape> ReduceTo<Self, AllAxes> for Tensor0D<H> {}
impl<H: Tape> BroadcastTo<Tensor0D<H>, AllAxes> for Tensor0D<H> {
    fn broadcast(self) -> Tensor0D<H> {
        self
    }
}

// 0d -> Nd
impl_broadcast_reduce!(Tensor0D<H>, AllAxes, Tensor1D<M, H>, {M});
impl_broadcast_reduce!(Tensor0D<H>, AllAxes, Tensor2D<M, N, H>, {M, N});
impl_broadcast_reduce!(Tensor0D<H>, AllAxes, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor0D<H>, AllAxes, Tensor4D<M, N, O, P, H>, {M, N, O, P});

// 1d -> Nd
impl_broadcast_reduce!(Tensor1D<M, H>, Axis<1>, Tensor2D<M, N, H>, {M, N});
impl_broadcast_reduce!(Tensor1D<N, H>, Axis<0>, Tensor2D<M, N, H>, {M, N});
impl_broadcast_reduce!(Tensor1D<M, H>, Axes2<1, 2>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor1D<N, H>, Axes2<0, 2>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor1D<O, H>, Axes2<0, 1>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor1D<M, H>, Axes3<1, 2, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor1D<N, H>, Axes3<0, 2, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor1D<O, H>, Axes3<0, 1, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor1D<P, H>, Axes3<0, 1, 2>, Tensor4D<M, N, O, P, H>, {M, N, O, P});

// 2d -> Nd
impl_broadcast_reduce!(Tensor2D<M, N, H>, Axis<2>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor2D<M, O, H>, Axis<1>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor2D<N, O, H>, Axis<0>, Tensor3D<M, N, O, H>, {M, N, O});
impl_broadcast_reduce!(Tensor2D<M, N, H>, Axes2<2, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor2D<M, O, H>, Axes2<1, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor2D<M, P, H>, Axes2<1, 2>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor2D<N, O, H>, Axes2<0, 3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor2D<N, P, H>, Axes2<0, 2>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor2D<O, P, H>, Axes2<0, 1>, Tensor4D<M, N, O, P, H>, {M, N, O, P});

// 3d -> 4d
impl_broadcast_reduce!(Tensor3D<M, N, O, H>, Axis<3>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor3D<M, N, P, H>, Axis<2>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor3D<M, O, P, H>, Axis<1>, Tensor4D<M, N, O, P, H>, {M, N, O, P});
impl_broadcast_reduce!(Tensor3D<N, O, P, H>, Axis<0>, Tensor4D<M, N, O, P, H>, {M, N, O, P});

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::AssertClose;
    use rand::thread_rng;

    #[test]
    fn test_valid_1d_broadcasts() {
        let _: Tensor1D<5> = BroadcastTo::<_, AllAxes>::broadcast(Tensor0D::zeros());

        let _: Tensor2D<5, 3> = Tensor1D::<3>::zeros().broadcast();
        let _: Tensor2D<5, 3> = Tensor1D::<5>::zeros().broadcast();

        let _: Tensor3D<3, 5, 7> = Tensor2D::<5, 7>::zeros().broadcast();
        let _: Tensor3D<3, 5, 7> = Tensor2D::<3, 7>::zeros().broadcast();
        let _: Tensor3D<3, 5, 7> = Tensor2D::<3, 5>::zeros().broadcast();
        let _: Tensor3D<3, 5, 7> = Tensor2D::<3, 5>::zeros().broadcast();

        let _: Tensor4D<3, 5, 7, 9> = Tensor3D::<5, 7, 9>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor3D::<3, 7, 9>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor3D::<3, 5, 9>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor3D::<3, 5, 7>::zeros().broadcast();
    }

    #[test]
    fn test_valid_2d_broadcasts() {
        let _: Tensor2D<5, 3> = BroadcastTo::<_, AllAxes>::broadcast(Tensor0D::zeros());

        let _: Tensor3D<3, 5, 7> = Tensor1D::<3>::zeros().broadcast();
        let _: Tensor3D<3, 5, 7> = Tensor1D::<5>::zeros().broadcast();
        let _: Tensor3D<3, 5, 7> = Tensor1D::<7>::zeros().broadcast();

        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<3, 5>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<3, 7>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<3, 9>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<5, 7>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<5, 9>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor2D::<7, 9>::zeros().broadcast();
    }

    #[test]
    fn test_valid_3d_broadcasts() {
        let _: Tensor3D<3, 5, 7> = BroadcastTo::<_, AllAxes>::broadcast(Tensor0D::zeros());

        let _: Tensor4D<3, 5, 7, 9> = Tensor1D::<3>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor1D::<5>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor1D::<7>::zeros().broadcast();
        let _: Tensor4D<3, 5, 7, 9> = Tensor1D::<9>::zeros().broadcast();
    }

    #[test]
    fn test_broadcast_backwards() {
        let mut rng = thread_rng();
        let a: Tensor1D<3> = TensorCreator::randn(&mut rng);
        let b: Tensor2D<5, 3> = TensorCreator::randn(&mut rng);
        let a_up: Tensor2D<5, 3, OwnedTape> = a.trace().broadcast();
        a_up.data().assert_close(&[*a.data(); 5], 1e-4);
        let r = mul(a_up, &b);
        let g = backward(r.exp().mean());
        // a's gradient: (b * (b * a).exp()).sum(0) / 15
        // b's gradient: (a * (b * a).exp()) / 15
        let a_up: Tensor2D<5, 3> = a.clone().broadcast();
        let a_grad = mul(mul(b.clone(), &a_up).exp(), &b).sum::<_, Axis<0>>() / 15.0;
        let b_grad = mul(mul(b.clone(), &a_up).exp(), &a_up) / 15.0;
        g.ref_gradient(&a).assert_close(a_grad.data(), 1e-4);
        g.ref_gradient(&b).assert_close(b_grad.data(), 1e-4);
    }
}
