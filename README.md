# pingmu
可以直接ping多个网段的二进制工具包

简单使用：第一个参数为ping的次数，然后可以添加任意个数的网段

`sudo ./pingmu -c 4 192.168.1.1/24 10.1.3.1-10.1.3.240  ...`

输出结果为：
```txt
192.168.1.1              [ 0.20ms, 0.14ms,]
.
.
.
192.168.1.10             [ 0.20ms, 0.14ms,]
10.1.3.240               [ idle, idle,]
+------------------+-------------------------------+---------------+--------------------+--------------------+
| loss ip/total ip | loss_packets/all_ping_packets | total_loss(%) | max delay(ex idle) | avg delay(ex idle) |
+------------------+-------------------------------+---------------+--------------------+--------------------+
| 248/258          | 498/516                       | 96.5116%      | 199.54ms           | 2.81ms             |
+------------------+-------------------------------+---------------+--------------------+--------------------+

```
更多参数使用说明：
`sudo ./pingmu -h`:
```bash
example:
 sudo ./pingmu 192.168.1.1/24 169.1254.169.254/32 ...
 sudo ./pingmu -c 10 -t 2000 -i 100 input.text out.csv nolog 192.168.1.1/30 10.0.0.1-10.0.0.5 127.0.0.1



+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+
|      |          | per ip times | TM(ms) | send i(us) | input.text | x.csv   | is log(op) | cidr|range|ip  | ...                       |
+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+
| sudo | ./pingmu | 4            | 2000   | 100        | input.text | out.csv | nolog      | 192.168.1.1/30 | 192.168.2.1-192.168.3.255 |
+------+----------+--------------+--------+------------+------------+---------+------------+----------------+---------------------------+

cat input.text
ip1
ip2
ip3
....

```

### help
```bash
sudo ./pingmu 
Usage: pingmu [OPTIONS] [cidr|range]...

Arguments:
  [cidr|range]...  <cidr | range> (s)  ex.  10.0.0.0/24 1.1.1.1-1.1.1.5

Options:
  -c, --count <count>              The number of icmp echo packges `ping`; [default: 3]
  -t, --timeout <timeout>          The Timeout for each icmp echo packge (/ms) [default: 1000]
  -i, --interval <interval>        The interval between sending every two packets (/ns) [default: 100]
  -l, --loglevel <loglevel>        Which log level to run [default: info] [possible values: off, error, warn, info, debug, trace]
      --input <ip list file path>  ip list input file like this: cat ips.txt: 192.168.1.2 \n 1.1.1.1 \n...   not support cidr or range
  -o, --outputpath <output path>   output each ping result to csv file. eg. output-xxxx.csv
  -h, --help                       Print help (see more with '--help')
  -V, --version                    Print version

eg. sudo ./pingmu  1.2.2.3/24 1.2.3.4-1.2.3.9
    sudo ./pingmu -c 1 1.2.3.4/30
    sudo ./pingmu -c 4 -input ips.txt -o output.csv 1.2.3.4/30

tips: 1. Need sudo.
      2. You can press ctrl+c anytime to stopped the program.
```


the out.csv: 
csv value example
```bash
out summary:
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

### 支持 linux/unix


todo list:
 - fixed x.x.x.0 ip -> done!
 - analysis the result to pretty table. -> done!
 - replace pnet lib cause only root user can do...
 - tcp ping
 - support ipv6
 