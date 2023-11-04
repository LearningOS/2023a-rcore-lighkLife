## 实现的功能
实现了一个可以获取当前任务信息的系统调用，该系统调用可以提供以下功能：
- 获取当前任务的运行状态
- 获取当前任务到目前为止运行的时间（毫秒）
- 获取当前任务所有的系统分别调用的次数

## 简答作业

### 三个 bad 测例

正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容
(运行`Rust 三个 bad 测例 (ch2b_bad_*.rs)`， 注意在编译时至少需要指定 `LOG=ERROR` 才能观察到内核的报错信息) ，
描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

```log
# ch2b_bad_address 用户程序尝试在 U 模式下向非用户内存地址写入数据，陷入了 StoreFault 异常（Store/AMO access fault）
[kernel] PageFault in application, bad addr = 0x0, bad instruction = 0x804003c4, kernel killed it.

# ch2b_bad_instructions 用户程序尝试在 U 模式下调用 S 模式特权指令 sret，陷入了 IllegalInstruction 异常
[kernel] IllegalInstruction in application, kernel killed it.

# ch2b_bad_register 用户程序尝试访问 S 模式 CSR 的指令
[kernel] IllegalInstruction in application, kernel killed it.
```

### 深入理解 `trap.S`

深入理解 `trap.S`中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:

#### 1. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。

- a0 指向内核栈的栈指针也就是我们刚刚保存的 Trap 上下文的地址；<br>
- `__restore` 会在系统启动时，内核第一次执行用户程序时，由 S 模式转为 U 模式时使用；
- `__restore` 会在系陷入（系统调研）执行完毕后，由 S 模式恢复到 U 模式时使用；

#### 2.L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

       ```riscv

       ld t0, 32*8(sp)
       ld t1, 33*8(sp)
       ld t2, 2*8(sp)
       csrw sstatus, t0
       csrw sepc, t1
       csrw sscratch, t2
       ```

| 符号       | 寄存器                                  | 意义                         |
|:---------|:-------------------------------------|:---------------------------|
| sstatus  | Supervisor Status Register           | 恢复 CPU 执行状态到 Trap 发生之前的用户态 |
| sepc     | Supervisor Exception Program Counter | 用作恢到用户态后，可以继续执行的下一条指令地址    |
| sscratch | Supervisor Scratch Register          | 作为中转寄存器，用作后续操作恢复用户栈地址      |

#### 3.L50-L56：为何跳过了 `x2` 和 `x4`？

        ```riscv

       ld x1, 1*8(sp)
       ld x3, 3*8(sp)
       .set n, 5
       .rept 27
       LOAD_GP %n
       .set n, n+1
       .endr
       ```

#### 4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

       `csrrw sp, sscratch, sp`
sp 指向用户栈栈顶，sscratch 指向内核栈栈顶

#### 5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？
`sret`中发生状态切换，因为该指令会将 `sepc`中的值设置到 `pc` 中，而`sepc`当前存放的就是 trap 之前，用户程序的下一条指令地址

#### 6. L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

       `csrrw sp, sscratch, sp`
sp 指向内核栈栈顶，sscratch 指向用户栈栈顶


#### 7. 从 U 态进入 S 态是哪一条指令发生的？
但用户程序调用 `ecall`时，从 U 态进入 S 态

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，<br>
   无 <br>
   还在代码中对应的位置以注释形式记录了具体的交流对象及内容：<br>
   无 <br>
2. 此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：<br>
   [1] [rCore-Tutorial-Book 第三版](https://rcore-os.cn/rCore-Tutorial-Book-v3/index.html)<br>
   [2] [RISC-V 手册](chrome-extension://cdonnmffkdaoajfknoeeecmchibpmkmg/assets/pdf/web/viewer.html?file=http%3A%2F%2Friscvbook.com%2Fchinese%2FRISC-V-Reader-Chinese-v2p1.pdf)<br>
   [3] [RISC-V 官网](https://riscv.org/technical/specifications/)
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定
   程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）
   复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何
   计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计