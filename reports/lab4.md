# CHAPTER 5 实验报告

## 编程作业
### 迁移维护前面章节的系统调用（略）
### 实现sys_linkat
> 思路：查找到old name的目录项，取出起指向的文件inode id，创建一个新的目录项，填入new name，并将其指向文件inode id
>
> linkat需要操作根目录，并且需要比较细粒度的更改data block的内容，因此具体实现的位置应该是在vfs层

本次实现采用“`syscall` 入口做参数处理，`fs` 层做语义实现，`easy-fs` 层做目录项操作”的分层方式。

- 在`os/src/syscall/fs.rs`中实现`sys_linkat`：
  - 使用`current_user_token + translated_str`将用户态字符串参数转换为内核可用的`&str`；
  - 做一些简单的检查；
  - 调用`fs`模块导出的`linkat`完成具体操作。
- 在`os/src/fs/inode.rs`中新增桥接函数：
  - `pub fn linkat(old_name: &str, new_name: &str)`；
  - 直接委托给`ROOT_INODE.linkat(...)`。
- 在`easy-fs/src/vfs.rs`中实现目录项追加：
  - 复用`find_inode_id(old_name, root_disk_inode)`方法找到旧文件inode id；
  - 以目录当前`size`作为插入到位置，确保push到目录的尾部；
  - 构造`DirEntry::new(new_name, old_inode_id)`并写入目录文件尾部，形成硬链接。

### 实现sys_unlinkat
> 思路：删除对应的目录项，如果发现链接到该文件的目录项数目为0，则需要删除文件，具体实现层也在vfs

- 在`os/src/syscall/fs.rs`中实现`sys_unlinkat`：
  - 用户参数字符串转换后直接调用`fs::unlinkat`。
- 在`os/src/fs/inode.rs`中新增桥接函数：
  - `pub fn unlinkat(name: &str)` -> `ROOT_INODE.unlinkat(name)`。
- 在`easy-fs/src/vfs.rs`中实现`Inode::unlinkat`：
  1. 先通过名字找到目标inode id；
  2. 遍历根目录统计指向该inode的目录项个数`cnt`，并记录待删除目录项下标`file_index`；
  3. 若`cnt > 1`：只把该目录项覆盖为 DirEntry::empty()`，即只删除这一个名字；
  4. 若`cnt == 1`：先定位inode并`clear()`清空文件数据，再将目录项置空。

> 感觉read_disk_inode和modify_disk_inode实现的特别丑，完全没必要传闭包进去。
> 遇到了一个坑： 
> 如果在持有`self.fs`锁时再调用`write_at`（而`write_at`内部也会加同一把锁），会出现卡住现象。后续通过缩小锁作用域（先取位置，释放锁，再执行后续写操作）避免了这个问题。

### 实现sys_fstat

> 思路：这个系统调用需要获取File对象的状态，也即只要实现了file trait的类型都应该实现这个系统调用，不仅包括block device，还包括stdin和stdout，因此实现的层数应该在trait层

- 在`os/src/fs/mod.rs`扩展`File` trait
- 在`os/src/fs/stdio.rs`中为 `Stdin/Stdout` 补齐该接口（题目没说清楚，因此默认不会获取stdin和stdout的状态，当前实现为`panic!`）。
- 在`os/src/fs/inode.rs`中为 `OSInode`实现`stat()`：
  - `ino`：通过inode的磁盘位置反推inode id（复用`find_inode_id`）；
  - `mode`：通过`is_dir/is_file`映射到`StatMode::{DIR, FILE}`；
  - `nlink`：遍历根目录目录项，统计inode id相同的条目数（`ROOT_INODE.get_nlink(ino)`）；
  - 其他字段置零
- 在`os/src/syscall/fs.rs`中实现`sys_fstat`：
  - 通过 `translated_refmut` 获取用户态 `Stat` 写回地址；
  - 从当前进程`fd_table`取出文件对象，调用`inode.stat()`写回。

## 问答作业
### 在我们的easy-fs中，root inode起着什么作用？如果root inode中的内容损坏了，会发生什么？

`root inode`是easy-fs的目录树入口，也是当前实验实现中绝大多数文件操作（`open/find/create/link/unlink/ls`）的起点。它的作用可以概括为：

1. **命名空间入口**：保存根目录下所有目录项（文件名 -> inode id 映射）；
2. **路径解析起点**：当前实验只处理根目录文件名，所有查找都从它开始；
3. **链接计数统计依据**：`nlink` 的计算依赖遍历根目录目录项；
4. **文件创建挂载点**：新文件的目录项要写入 root inode 对应目录数据区。

如果root inode内容损坏：无法获取文件系统里真实的文件信息，但笔者认为可以修复。在已知文件系统的情况下，可以通过遍历每个block的方法识别所有正常的文件，即可重建root inode。
