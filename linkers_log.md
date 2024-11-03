### While trying to set up a minimal Linux system, I noticed that there are no linkers written in Rust:
> 15/09/2024

That makes sense because Rust's "libraries" are statically compiled into the binary, but it's nonetheless an affront to everything I stand for...

I have no experience with shared objects, linking, loading, and the like... but that's just what will make this a fun project, no going over things I already know. It's time to start over (again); I am getting pretty good at reading these PDFs from the 90s.

How hard can it be anyway. (That's called preemptive sarcasm)


### Nice thing about Rust developers is they know how to use actual fucking variable names:
> 18/09/2024

I am looking at you c devs working on musl:

```c
#if DL_FDPIC
    for (i=0; i<DYN_CNT; i++) {
        if (i==DT_RELASZ || i==DT_RELSZ) continue;
        if (!dyn[i]) continue;
        for (j=0; dyn[i]-segs[j].p_vaddr >= segs[j].p_memsz; j++);
        dyn[i] += segs[j].addr - segs[j].p_vaddr;
    }
    base = 0;

    const Sym *syms = (void *)dyn[DT_SYMTAB];

    rel = (void *)dyn[DT_RELA];
    rel_size = dyn[DT_RELASZ];
    for (; rel_size; rel+=3, rel_size-=3*sizeof(size_t)) {
        if (!IS_RELATIVE(rel[1], syms)) continue;
        for (j=0; rel[0]-segs[j].p_vaddr >= segs[j].p_memsz; j++);
        size_t *rel_addr = (void *)
            (rel[0] + segs[j].addr - segs[j].p_vaddr);
        if (R_TYPE(rel[1]) == REL_FUNCDESC_VAL) {
            *rel_addr += segs[rel_addr[1]].addr
                - segs[rel_addr[1]].p_vaddr
                + syms[R_SYM(rel[1])].st_value;
            rel_addr[1] = dyn[DT_PLTGOT];
        } else {
            size_t val = syms[R_SYM(rel[1])].st_value;
            for (j=0; val-segs[j].p_vaddr >= segs[j].p_memsz; j++);
            *rel_addr = rel[2] + segs[j].addr - segs[j].p_vaddr + val;
        }
    }
#else
```

Like what the fuck people:

```c
static void decode_vec(size_t *v, size_t *a, size_t cnt)
{
    size_t i;
    for (i=0; i<cnt; i++) a[i] = 0;
    for (; v[0]; v+=2) if (v[0]-1<cnt-1) {
        if (v[0] < 8*sizeof(long))
            a[0] |= 1UL<<v[0];
        a[v[0]] = v[1];
    }
}
```

We aren't playing code golf...


After raging on my own, I found someone with the same experience. Unfortunately, they gave up 8 years ago: [https://github.com/m4b/dryad/issues/5#issuecomment-262696880]

To quote m4b: "I've jokingly told people I'm worried what will happen when all the old C programmers die - but I'm not really joking."

We are 8 years closer to that reality, and I can't get a job, so fuck it!


### I am starting to see why these codebases (glibc, musl, etc...) are so hard to read as a beginner:
> 03/10/2024

In programming, we usually rely on abstraction to simplify our jobs. We make structures to contain and compartmentalize complex or dangerous code. However, at this level, those just obfuscate an already confusing system.

You start to find looking up definitions annoying; it's a null terminated list of elements; just treat it that way. On top of that, laziness works in that ideas favor:

```rs
pub(crate) const AT_NULL: usize = 0;
pub(crate) const AT_PAGE_SIZE: usize = 6;
pub(crate) const AT_BASE: usize = 7;
pub(crate) const AT_ENTRY: usize = 9;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorItem {
    pub a_type: usize,
    pub a_val: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct AuxiliaryVectorIter(*const AuxiliaryVectorItem);

impl AuxiliaryVectorIter {
    pub(crate) fn new(auxiliary_vector_pointer: *const AuxiliaryVectorItem) -> Self {
        Self(auxiliary_vector_pointer)
    }

    pub(crate) fn into_inner(self) -> *const AuxiliaryVectorItem {
        self.0
    }
}

impl Iterator for AuxiliaryVectorIter {
    type Item = AuxiliaryVectorItem;

    fn next(&mut self) -> Option<Self::Item> {
        let this = unsafe { *self.0 };
        if this.a_type == AT_NULL {
            return None;
        }
        self.0 = unsafe { self.0.add(1) };
        Some(this)
    }
}
```

The truth is you are only going to use that struct 3 or so times; why not just write:

```rs
(0..).map(|i| unsafe { *auxiliary_vector_pointer.add(i) }).take_while(|t| t.a_type != AT_NULL)
```

The same goes for naming; is `AuxiliaryVector` really any more helpful than `auxv`? Many of these things you can only find in pdfs from the 90s; if you change the name to something more descriptive, you run the risk of no one being able to understand you.

Either way, what is a more descriptive name? It's just an assortment of possibly useful stuff passed to the linker by the Linux kernel... You are going to have to look it all up anyway.

### Anyone who thinks AI is going to take over the world has not had it insist that the reason your code is not working on a 64-bit arch is because you are using a usize, not a u64... (>ლ)
> 17/10/2024

Any way, I fixed that issue, but now I have a new one... And it's showing me why this type of code is still written in C:

- **High-level Rust:** If it compiles, it works (most of the time).
- **C:** Your code does what you tell it to... even if you are stupid.
- **Low-level Rust:** Just because your code compiles and is safe doesn't mean it works... Also, the compiler won't do what you want.

After another half hour I changed this code:

```rs
dynamic_array_iter.for_each(|i| match i.d_tag {
    DT_RELA => rela_pointer = unsafe { base.byte_add(i.d_un.d_ptr.addr()) } as *const Rela,
    DT_RELASZ => {
        rela_count = unsafe { i.d_un.d_val } / core::mem::size_of::<Rela>();
    }
    #[cfg(debug_assertions)]
    DT_RELAENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Rela>()),
    // other stuff we may need:
    DT_PLTGOT => global_offset_table = unsafe { base.byte_add(i.d_un.d_ptr.addr()) },
    DT_SYMTAB => symbol_table = unsafe { base.byte_add(i.d_un.d_ptr.addr()) },
    #[cfg(debug_assertions)]
    DT_SYMENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Symbol>()),
    _ => (),
});
```

To this:

```rs
for i in dynamic_array_iter {
    match i.d_tag {
        DT_RELA => rela_pointer = unsafe { base.byte_add(i.d_un.d_ptr.addr()) } as *const Rela,
        DT_RELASZ => {
            rela_count = unsafe { i.d_un.d_val } / core::mem::size_of::<Rela>();
        }
        #[cfg(debug_assertions)]
        DT_RELAENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Rela>()),
        // other stuff we may need:
        DT_PLTGOT => global_offset_table = unsafe { base.byte_add(i.d_un.d_ptr.addr()) },
        DT_SYMTAB => symbol_table = unsafe { base.byte_add(i.d_un.d_ptr.addr()) },
        #[cfg(debug_assertions)]
        DT_SYMENT => syscall_assert!(unsafe { i.d_un.d_val } as usize == size_of::<Symbol>()),
        _ => (),
    }
}
```

And now it all works fine... I don't know if that just changed the layout or if closures aren't safe.
If it's closures then we might have some issues because I use a lot of them!

### (48) days of no_std
> 2/11/2024

I finally can use the full standard library... Now I just need to actually do linking.
The main fix was to just statically link the C runtime library; otherwise it takes too much work and runtime to handle everything.

That leaves us to handle two relocation types which is easy:

```rs
R_X86_64_RELATIVE => {
    let relocate_value = base_address.wrapping_add_signed(rela.r_addend);
    asm!(
        "mov qword ptr [{}], {}",
        in(reg) relocate_address,
        in(reg) relocate_value,
        options(nostack, preserves_flags),
    );
}
R_X86_64_IRELATIVE => {
    let function_pointer = base_address.wrapping_add_signed(rela.r_addend) as *const ();
    let function: extern "C" fn() -> usize = core::mem::transmute(function_pointer);
    let relocate_value = function();
    asm!(
        "mov qword ptr [{}], {}",
        in(reg) relocate_address,
        in(reg) relocate_value,
        options(nostack, preserves_flags),
    );
}
```

Anyway I need to sleep. Good night!
