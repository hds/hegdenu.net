+++
title = "Initial commit"
author = "hds"
date = "2022-08-10"
+++

The first post on [hēg denu](https://hegdenu.net).

Here's some code to look at.

```perl
#!/usr/bin/perl
chop($_=<>);@s=split/ /;foreach$m(@s){if($m=='*'){$z=pop@t;$x=
pop@t;$a=eval"$x$m$z";push@t,$a;}else{push@t,$m;}}print"$a\n";
```

