# A worked run

The three steps on a real question, with the output as printed on 2026-10-14 (trimmed
for length).

## Locate

```
$ librarian query software "how does a token bucket rate limiter work?" --limit 8

[0.677] software/markdown/networking-dordal/chapter-19-queuing-and-scheduling.md#53
  Dordal > Chapter-19-Queuing-and-Scheduling > 19.9 Token Bucket Filters > 19.9.1 Token Bucket Definition
  The idea behind a token bucket is that there is a notional bucket somewhere, being
  filled at a steady rate with tokens ...

[0.673] software/markdown/box/an-introduction-to-computer-networks.md#1248
  box > An_Introduction_to_Computer_Networks > 19.9 Token Bucket Filters > ...

[0.656] software/markdown/box/system-design-interview-an-insiders-guide-2.md#57
  box > system-design-interview-an-insiders-guide-2 > CHAPTER 3 > Algorithms for rate limiting
  Rate limiting can be implemented ...

[0.650] software/markdown/networking-dordal/chapter-19-queuing-and-scheduling.md#54
  ...

(four more hits, all from the same three books)

confidence: WEAK (0.68)  [top 0.677, margin 0.003, fragments 0%]
```

Three things to notice. The question retrieved explanatory prose, not headings
(`fragments 0%`). The first and second hits are the same Dordal chapter indexed twice,
once as a chapter file and once inside the whole-book file, which is why the margin is
0.003 and the label is `WEAK` despite a strong top score. And the hits already show two
different treatments, a networking textbook and a system-design book, which matters if
the user's context is one or the other.

## Read

The top hit is a definition paragraph. Pull its neighbours to get the full treatment:

```
$ librarian extract software "software/markdown/networking-dordal/chapter-19-queuing-and-scheduling.md#53" --context 1

#52  ... A token-bucket filter can be thought of as a queuing discipline, with an
     underlying FIFO queue. If noncompliant packets are delayed, it is
     non-work-conserving. Dropping non-compliant packets can be viewed as an
     alternative to tail-drop ...

#53  The idea behind a token bucket is that there is a notional bucket somewhere, being
     filled at a steady rate with tokens (or, if more divisibility is needed, with
     fluid); any overflow from the bucket is discarded. To send a packet, we need to be
     able to take one token from the bucket; if the bucket is empty then the packet is
     non-compliant ... More precisely, a token-bucket specification TB(r,Bmax) includes
     a token fill rate of r tokens/sec ... and also a bucket capacity (or depth)
     Bmax>0 ... If the bucket is full, however, then the sender may send a burst of
     packets corresponding to the bucket capacity ...

#54  Two token-bucket-compliant senders are shown ... When I first got satellite
     Internet, my service was limited by a token-bucket filter with rate 56 kbps and
     bucket 300 megabytes ...
```

Chunk 52 gives the preceding discussion of what happens to non-compliant packets, 53
the definition with the TB(r, Bmax) notation and 54 a worked illustration. One extract
turned a pointer into a passage.

## Report

What the user sees:

> A token bucket is a notional bucket filled with tokens at a steady rate r and capped
> at a depth Bmax, with any overflow discarded. Sending a packet of size S needs S tokens
> in the bucket. If they are there the packet is compliant and the tokens are removed;
> otherwise it is non-compliant and is delayed, dropped or marked according to policy.
> A full bucket therefore permits a burst of up to Bmax before the sender is held to the
> rate r. (Dordal, *An Introduction to Computer Networks*, 19.9.1 Token Bucket
> Definition, `software/markdown/networking-dordal/chapter-19-queuing-and-scheduling.md#53`)
>
> The library supports this solidly. Retrieval reported WEAK, but only because the same
> chapter is indexed twice and the two copies tied; the passage itself is a direct
> definition. The system-design interview book has a shorter, implementation-oriented
> version if you want that angle.

The confidence sentence is not optional. The user cannot see the retrieval, so the
report is their only signal of how grounded the answer is.
