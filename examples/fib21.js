const start = performance.now();

let a = 0;
let temp;

for (let b = 1; a < 10000; b = temp + b) {
  console.log(a);
  temp = a;
  a = b;
}

const end = performance.now();

console.log("time spent:");
console.log(end - start);
