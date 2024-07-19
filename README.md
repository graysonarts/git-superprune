# git-superprune

A git command to prune remote branches that have already been merged.

## What it automates

```
> git branch -v

  add-sstripper-to-video-processor                                        5809024 Merge pull request #39 from 1
  create_additional_clips                                                 2e58682 [gone] Handle fetching information
  enable-logging-for-debug-mode                                           d6f5a52 checkpoint
  fix-segmentation                                                        de3d099 [gone] move message to debug
  grayson/elk-50-make-jobid-something-that-the-workflows-are-aware-of-and 04821cc [gone] fix the job status updating
  job_watcher                                                             e8bfa01 [gone] adding instructions
* main                                                                    8dcade5 Merge pull request #58 from 2
  rate-limit-handling                                                     7782767 checkpoint
  tweak_autoscale_for_demo                                                23a5234 [gone] Adjust scaling for demo

> git branch -d tweak_autoscale_for_demo
...
```

## Usage

```
> git superprune
```

This is a dry run that prints out which branches will be deleted.

```
> git superprune -x
```

Delete the branches

```
> git superprune -h
