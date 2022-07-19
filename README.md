# pingmu
 ICMP ping library in Rust inspired by go-fastping and AnyEvent::FastPing Perl module

`sudo ./pingmu -h`:
```bash

+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+
|      |          | per ip times | TM(ms) | send i(us) | input.text | x.csv   | is log(op) | cidr|range|ip  | ...                       |
+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+
| sudo | ./pingmu | 4            | 2000   | 100        | input.text | out.csv | nolog      | 192.168.1.1/30 | 192.168.2.1-192.168.3.255 |
+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+
example:
sudo ./pingmu 10 2000 100 input.text out.csv nolog 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1

cat input.text
ip1
ip2
ip3
....

```


the out.csv: 
csv value example
```bash
out.csv: value example
+--------------+---------+---------+---------+---------+--------+---------+
| ip           | loss(%) | min(ms) | avg(ms) | max(ms) | stddev | ...(ms) |
+--------------+---------+---------+---------+---------+--------+---------+
| 192.168.10.x | 0%      | 5.93    | 15.37   | 20.00   | 32     | 10      |
+--------------+---------+---------+---------+---------+--------+---------+
```

fork from fasting-rs
Only supported on linux and osx for now (Windows will likely not work).  


### static build:
```shell
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```


todo list:
 - fixed x.x.x.0 ip
 - analysis the result to pretty table.
 - ...
