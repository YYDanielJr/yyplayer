# 主分支统一记录

2026-10-02：用户要求合并 main / master，名称任意；沿用已经准备好的 master。

操作前：工作区干净，无远端；main=4eca722，master=dev=f01498f。`git merge-base --is-ancestor main master` 返回 0；`git log --left-right main...master` 仅有 master 一侧 6 个提交，无 main 独有提交。只有一个工作树，当前 master。

执行：

```powershell
git merge --ff-only main
git branch -d main
git diff --check
git add README.md AGENTS.md docs/PUBLISHING.md docs/STATUS.md docs/validation/branch-consolidation.md
git commit -m "Consolidate primary branch as master"
git switch dev
git merge --ff-only master
git switch master
```

结果：merge 已全部包含，安全删除重复本地 main，master / dev 保留并同步，当前 master。提交 4eca722 仍是 master 祖先；没有改写提交历史、删除代码、修改远端或推送。最终 status 干净。

仅 Git / 文档整理，应用源码和成品未变；不重复编译、单测或物理播放资格检查。发布根目录仍为 E:\SourceFiles\rust\yyplayer，使用 VS Code 发布 master。
