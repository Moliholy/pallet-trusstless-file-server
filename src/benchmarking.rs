//! Benchmarking setup for pallet-trustless-file-server
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as TrustlessFileServer;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    // from 1KB to 32KB
    #[benchmark]
    fn upload_file(x: Linear<1024, 32768>) {
        let caller: T::AccountId = whitelisted_caller();
        let bytes = vec![(x % u8::MAX as u32) as u8; x as usize];

        #[extrinsic_call]
        _(RawOrigin::Signed(caller.clone()), bytes);

        assert!(Files::<T>::iter().next().is_some());
    }

    impl_benchmark_test_suite!(TrustlessFileServer, crate::mock::new_test_ext(), crate::mock::Test);
}
