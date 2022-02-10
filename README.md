# pingmu
 ICMP ping library in Rust inspired by go-fastping and AnyEvent::FastPing Perl module

`sudo ./pingmu -h`:
```bash
do not > 4w ips
+------+--------+----------------------+----------+----------------+--------+
|      |        | ping number of times | filename | cidr or range  |  or ip |
+------+--------+----------------------+----------+----------------+--------+
| sudo | pingmu | 4                    | out.csv  | 192.168.1.1/30 |        |
+------+--------+----------------------+----------+----------------+--------+
example:
sudo pingmu 10 out.csv 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1
```


the out.csv: 
csv value example
```bash
+--------------+---------+---------+---------+---------+--------+---------+
| ip           | loss(%) | min(ms) | avg(ms) | max(ms) | stddev | ...(ms) |
+--------------+---------+---------+---------+---------+--------+---------+
| 192.168.10.x | 0%      | 5.93    | 15.37   | 20.00   | 32     | 10      |
+--------------+---------+---------+---------+---------+--------+---------+
```

fork from fasting-rs
Only supported on linux and osx for now (Windows will likely not work).  


todo list:
 - fixed x.x.x.0 ip
 - analysis the result to pretty table.
 - ...
