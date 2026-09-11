


## 6. Lessons Learned

1. Mathematics != Implementation: A correct formula can still produce catastrophic 
   failure if the surrounding infrastructure violates memory safety contracts.

2. Zero-Copy is Non-Negotiable: The 9000x latency improvement came from eliminating 
   heap allocation entirely, not optimizing the algorithm itself.

3. Invariants Must Be Enforced at Compile Time: Runtime checks for NaN are useless if 
   the code path that produces them has already crashed with UB.


## References
- Legacy Source: dev/legacy_staging/aas_omni_galaxy/
- Rebased Code: crates/compute/src/relevancy_filter.rs
- Next Case Study: See 0002_<name>.md

