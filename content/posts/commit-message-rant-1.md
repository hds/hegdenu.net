+++
title = "commit message rant (part 1 of n)"
slug = "commit-messages"
author = "hds"
date = "2024-03-20"
draft = true
+++

The other day I was setting up release automation for a Rust project. Everything was going great and I'm happy with the release tooling I'm trying out. Then it got to creating the release PR. This looks great, it includes all the information about the release. The version that's being released, a changelog (which is customizable), as well as a link to the commits in the latest version. Here's an example from a [test project](https://github.com/hds/hds_test0):

![Screenshot of a GitHub Pull Request description. It specifies that the crate `hds_test0` will go from version 0.0.3 to 0.0.4 and provides a list of changes of which there are 2.](/img/commit-message-rant-1/hds_test0-v0.0.4-release-pr.png)

Fantastic, all the information that I need in one place. A summary of the changes that have gone into that release as well as the version number that these changes will be released with. Then I go to the subsequent commit message and it looks like this:

```COMMIT_EDITMSG
chore: release (#6)

Signed-off-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
Co-authored-by: github-actions[bot] <41898282+github-actions[bot]@users.noreply.github.com>
```

All that wonderful information, all that rich context, gone! Blown away onto the wind. Or rather, trapped in a dark room with a door that only sort of works.

Now's the part where I have to apologise to [Marco Ieni](https://www.marcoieni.com/), the author of the fantastic [release-plz](https://release-plz.ieni.dev/) project. I don't want to take aim at Marco specifically, it was just that this experience perfectly highlighted the general trend to not include important information in commit messages.

> Note to self: open an issue on release-plz to include more detailed information in the commit message.

### rant

This is a long coming rant, which may be the first of various, but be warned that it is a bit, ... ranty. Don't say I didn't warn you.

These days, I would wager that a very large percentage of the readers of this site use [Git](https://git-scm.com/), for better or worse it has become ubiquitous in much of the industry and perhaps even more so in the open source world. I'm going to use Git as an example throughout this post, but everything I say applies to every other source code management / version control system that is worth using. I'd even go so far as to say that any system that doesn't allow the sort of access to commit messages that I'll describe is actively working against your best interests.

Commit messages are the most durable store of information that any software project has. When you clone (or checkout) a project, the commit messages are right there. Every member of your team has a local copy. Accessing commit messages is simple and extracting them from the rest of the repository is not much more complicated.

So why would you waste this fantastic store of information with commit messages like:

```COMMIT_EDITMSG
Fix NullPointerException
```

Sorry! This is a Rust blog:

```COMMIT_EDITMSG
Fix unwrap panic
```

You may laugh and say no one ever does this. But I did a search on a private GitLab instance I have access to and found 2.5K commits where the message was some variation of this with no more information! Interestingly I found a few results for "panic" as well, but the results were a little more varied (some of them were related to aborting on panic and many more related to terraform). Still, very few had any actual commit message. Part of the fault of this is GitLab itself, but we'll go into that later.

This isn't very useful, I could probably work out for myself that a ~~`NullPointerException`~~ panic was being fixed from the code. What is interesting is why this change was needed. What are the assumptions which were previously made, but have now been discovered to be incorrect? This is the information that will be useful both for the code review, but also later on once everyone has forgotten.

### what should a commit message contain?

In one word: **context**.

This topic was covered wonderfully by [Derek Prior](https://www.prioritized.net/contact/) (the principal engineering manager at GitHub, not the fantasy book author by the same name) in his 2015 RailsConf talk [Implementing a Strong Code-Review Culture](https://www.youtube.com/watch?v=PJjmw9TRB7s). If you haven't seen that talk, it is well worth watching.

To summarise, a commit message should contain the **why** and the **what**. Why was a change necessary? Why was it implemented the way it was? Why were the tools used chosen? What was changed? What benefits and and down-sides does the implementation have? What was left out of this particular change? (and why?)

If you're the sort of person who writes a single line summary and leaves it at that (we've all been that person), start by making yourself write two paragraphs in the body of the commit message for every commit. (1) Why was this change made. (2) What does this change do.

And all of this should be in the **commit message**. (want to see an [example](#show-me-an-example)?)

You should also definitely link the issue, ticket, or whatever it is that you use to prioritize work. But that is part of the why, not all of it.

And yes, I can hear many of you saying...

### but it's already somewhere else!

There are people screaming, this is already written down! It's in the ticket! It's in the Pull Request description! It's written on a sticky note on the side of the server! (you'd be surprised)

I'm sure you have this information written down, but there are two reasons why the commit message is a much better place for this information - even if that means duplicating it.

The first is persistence. As mentioned above, commit history is a distributed store of information, there are redundant copies on every developer's machine. It doesn't matter if you lose your internet connection, you've still got the commit history and all those wonderful commit messages.

Your ticketing system does not have these properties. [GitHub](https://github.com/) (or [GitLab](https://gitlab.com) or [Codeberg](https://codeberg.org/)) does not have these properties.

I've seen JIRA projects get deleted for all sorts of reasons. It's confusing keeping this old project around, people will create tickets for us there, let's just delete it. We're migrating to a new instance with a simpler configuration, migrating all the tickets is too complex, it's better to start afresh. JIRA is too complex, we're moving to a simpler solution that covers all our needs, no, we can't import our closed tickets.

GitHub has been a staple of open source development for a decade and a half now. But many open source projects have lived much longer than that. GitHub won't be around for ever, and when it comes time to migrate to whatever solution we find afterwards, pulling all the PR and Issue descriptions out of the API is likely to be something that many people simply don't have time for. 

I challenge you to find a semi-mature engineering team that will accept migrating to a new version control system that doesn't allow them to import their history from Git.

Keeping that valuable information behind someone else's API when you could have it on everyone's dev box seems crazy.

The second reason is cognitive. There is no person in the history of time and the universe who understands a change better than the you who just finished writing it and runs `git commit`. So this is the person who should describe the changes made. Not the Product Owner who wrote the user story, not the Principal Engineer who developed the overall architecture, you the developer who just finished writing the code itself.

If you amend your commit as you work, then you can amend the message as well, keeping it up to date with the changes in that commit. If you prefer to develop in separate commits, then ensure that each commit contains a full picture of the code up until that point. You don't want to be scratching your head trying to remember why you picked one of three different patterns for that one bit of logic you wrote last week.

A Pull Request description has many of these benefits as well, but it lacks the persistence and accessibility as mentioned above.

### the tooling is fighting you

Like all those social media and app store walled gardens that we love to hate, source code management software and ticketing systems want to lock you in.

Aside from the obtuse APIs they often provide to access this data, some of them are actively "forgetting" to add important information to merged commit messages.

[GitHub](https://github.com/) encourages you to add your commit message as the PR description - but only the first commit when the PR is created. Then the default merge (as in merging a branch) message contains none of that PR description - so you're left with whatever development history the branch has. And "`Fixed tests`" is not a useful commit message to find anywhere.

At least GitHub squash merges include all the commit messages of the squashed commits by default (as little use as that often is). By default, when [GitLab](https://gitlab.com) creates a squash merge it will include the summary taken from the Merge Request title and then for the message body: **nothing at all**! This is actually one of the reasons why my search results turned up so many commits with no message body.

Ironically (because people love to hate it), [Gerrit](https://www.gerritcodereview.com/) is the one piece of source code management software that does commit messages correctly. Commit messages show up as the first modified file in a changeset. The commit message can be commented on like any changed file. When the merge occurs, the commit message that has been reviewed is what gets included.

Linking to a ticket (issue) is a good idea. But also mention other tickets related to previous or upcoming changes that the implementation had to take into consideration. When you lose access to those tickets, this extra information can help find other relevant changes as your poor successors (maybe including future you) are going about [software archaeology](https://en.wikipedia.org/wiki/Software_archaeology).

### show me an example

Let's looks at an example.

```COMMIT_EDITMSG
TSERV-2140: Add Envoy
```

We have a ticket number, so maybe there's some useful information there. Oh, too late, the ticketing system was migrated 3 years ago, we didn't keep old tickets.

Wouldn't it be better if we had a bit more **context**?

```COMMIT_EDITMSG
TSERV-2140: Add TCS load balancing with Envoy
 
Use Envoy proxy (https://www.envoyproxy.io/) in a container sidecar to
perform client side load balancing of gRPC requests from Trasa to the
deployed Traffic Cache Service (currently Zamyn).
 
In order to support high availability (99.9% SLA), we require replicas
of both the traffic cache service pods as well as the front-end trasa
pods.
 
Load balancing gRPC connections from the trasa pods to the TCS pods
isn't something that is supported natively by Kubernetes (see
investigation in TSERV-2138), so we require another solution.
 
This change adds an additional container to the Trasa pods containing
Envoy proxy. Envoy handles service discovery via the Kubernetes service
DNS record as well as request level load balancing of HTTP/2 (which lies
underneath gRPC). The Trasa container now connects to the Envoy
container (via the pod-internal localhost interface) and Envoy connects
to all the traffic cache service pods.
```

