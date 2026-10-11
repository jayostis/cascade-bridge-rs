const variants = { plain: (n)=>new WebAssembly.Memory({ initial: n }),
  max: (n)=>new WebAssembly.Memory({ initial: n, maximum: 65536 }),
  shared: (n)=>new WebAssembly.Memory({ initial: n, maximum: 16384, shared: true }) };
for (const [name, mk] of Object.entries(variants)) for (const size of [100, 1000, 5000]) {
  const m = mk(size); const t = performance.now();
  for (let i = 0; i < 200; i++) m.grow(1);
  console.log(`${name} grow(1) at ${size*64/1000} MB: ${((performance.now()-t)/200).toFixed(3)} ms`);
}
