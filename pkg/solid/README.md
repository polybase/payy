# Solid Consensus Protocol

Protocol for achieving high throughput BFT consensus for a given state machine.

Features:
 
 ✅ Fast finality (finality after 2 blocks)

 ✅ High throughput (minimal communication, pipelining)

 ✅ Safety over liveness

 ✅ BYO Network/state machine 



## Background

[Consensus]((https://www.youtube.com/watch?v=rN6ma561tak&list=PLeKd45zvjcDFUEv_ohr_HdUFe97RItdiB&index=18)) is a mechanism for converging to an agreement on some state in a distributed system. [BFT](https://www.youtube.com/watch?v=LoGx_ldRBU0&list=PLeKd45zvjcDFUEv_ohr_HdUFe97RItdiB&index=5) (Byzantine Fault Tolerance) consensus provides a provable mechanism of ensuring that network state is consistent so long as no more than 1/3 of nodes are faulty/malicious.


## Overview of Solid

Solid prioritizes safety over liveness (similar to [Doomslug](https://near.org/blog/doomslug-comparison), [HotStuff](https://hackernoon.com/hotstuff-the-consensus-protocol-behind-safestake-and-facebooks-librabft) and [Tendermint](https://docs.tendermint.com/v0.34/introduction/what-is-tendermint.html#consensus-overview)). 


Solid minimizes network communication. `accept` messages are sent only to the next designated leader (many to one). Once a leader accumulates `accept` messages from a majority of nodes (including itself), 
it will then propose a new `proposal` to the network (one to many).

If the network detects that no proposal has been received from the network within a given timeout period, then the node will then send an `accept` to the next designated leader, with a `skip` of 1. This process is repeated until a valid proposal is sent from a node.

This is how a typical network using Solid might look like:

![Consensus Protocol Overview](docs/consensus.png)

Solid implements an async event stream interface where relevant events are omitted and it is the responsibility of the implementor to define the network and storage implementations.

The number of nodes should always be an odd number to prevent a deadlock situation where no majority can be reached. This is the *liveness* property alluded to previously.



### Terminology

  * Leader - The node which is in charge of submitting proposals. See [Leader Election](#leader-election)

  * Proposal - a request to add a block with a bunch of transactions to the network.

  * Accepts - message sent by all nodes to the leader indicating acceptance of the previous proposal. Also sent in the case where a leader is skipped since
              it did not produce a proposal in time.

  * Skips - the number of changes of leaders since the last confirmed proposal. A leader is skipped if it does not produce a proposal before timeout.

  * Height - the number of committed blocks on the network. A proposal has an associated height which indicates how updated (or outdated) the node state is.

  * Validators - all the nodes of the system that vote for a proposal to be accepted (so all the available, non-malicious nodes in the system).


### Solid State Machine and Transitions

```mermaid
%% Note: This content is sourced from docs/solid.mmd. See the instructions in that file. Any modifications to the code
%% must be done in that file, and the changes copied over here.
stateDiagram-v2
  state CheckStartState <<choice>>
  state CheckProposalHeight <<choice>>
  state CheckAcceptedByQuorum <<choice>>
  state CheckHasPendingCommits <<choice>>

  [*] --> CheckStartState: Starting state
  CheckStartState --> Genesis: New node startup
  CheckStartState --> WithLastConfirmedProposal: Has last confirmed proposal
  Genesis --> NewRound
  WithLastConfirmedProposal --> NewRound

  NewRound --> Propose

  state CheckDuplicateProposal <<choice>>
  Propose --> CheckDuplicateProposal
  CheckDuplicateProposal --> DuplicateProposal: proposal is duplicate
  DuplicateProposal --> ResetTimeouts
  ResetTimeouts --> SkipAndChooseNewLeader
  SkipAndChooseNewLeader --> NewRound
  CheckDuplicateProposal --> CheckProposalHeight: proposal is not duplicate
  CheckProposalHeight --> Accept: height < proposal height


  Accept --> CheckAcceptedByQuorum
  CheckAcceptedByQuorum --> ResetTimeouts: proposal not accepted by majority of validators 
  CheckAcceptedByQuorum --> Commit: proposal accepted by majority of validators

  Commit --> CheckHasPendingCommits
  CheckHasPendingCommits --> OutOfSync: if has pending commits
  OutOfSync --> SyncWithNetwork
  SyncWithNetwork --> NewRound
  CheckHasPendingCommits --> IncrementHeight: if no pending commits
  IncrementHeight --> NewRound
```

### Solid States (and Conditions)

Note that the actual states that a node can be in are:

  * Propose
  * Accept, and
  * Commit

In addition, we consider a number of other pseudo-states (basically, events):

  * Out of Sync
  * Out of Date
  * Duplicate Proposal

Also note that these states and conditions are applicable to each node of the network at different stages of operation meaning that the states are not specific to any particular node unless explicitly
noted as such, and any particular node can be in any of the states ate different points in time.

The Solid states in the state transition diagram shown above are explained in the following sections. 

#### Propose

Only the leader node (i.e., the single node amongst all the nodes which has been chosen for this round) can put forth a proposal.

A  proposal has the following general structure:

  * last_proposal_hash - the hash of the last proposal.
  * skips - the number of changes of leader that have occurred since the last leadership change.
  * height - the height of the proposal (meaning the number of blocks that have already been committed in the network).
  * leader_id - the id of the proposer/leader (See also [Leader Election](#leader-election)).
  * txns - the changes included in the proposal, and 
  * peers - the list of peers on the network

The `txns` field is generic - any higher-level protocol (i.e., any solution built on top of Solid) can specify its own transaction format and thereby propose arbitrarily complex payloads.

If the leader node does not have any proposals, or if the leader fails to put forth the proposal before the timeout period, the round is skipped and a new round is started with *possibly* a new leader. See [Leader Election])#Leader election).

#### Accept

Accepts are sent to the next designated leader in the following scenarios: 
 
  1. When a valid proposal is received from the previous leader (building on the last confirmed proposal)

  2. After a given timeout (in order to move on from nodes that are not online), these are referred to as `skips`

  3. If there are no pending proposals (usually on start up)

Accepts are either for a proposal:
    - confirmed height + 1 (when a valid proposal is received)
    - confirmed height (when no valid pending proposals exist)

#### Commit

In the commit state, the proposal gooes through a final round of validation - if the node has pending commits in its local store/register/cache, then the node is considered to be out of sync, and as such transitions to the [Out of Sync](#out-of-sync) state. If not,
the commit is carried through and the `height` is incremented by 1 in preparation for the next round.

#### Out of Sync

This condition implies that the node's state is behind that of the network. As such, Solid will notify the higher-level protocol, which is then responsible for handling this conditions. For instance, `Polybase` will handle this condition
by attempting to update the state of this node by accepting snapshots from other nodes in the network.

#### Out of Date

This condition implies that the proposal is out-of-date or invalid, and as such, Solid again delegates the handling of the proposal to the higher-level protocol. For instance, `Polybase` will log and discard the proposal.

#### Duplicate Proposal

This condition implies that the proposal has already been seen by the network, and as such, Solid will delegate the appropriate follow-up to the higher-level protocol. For instance, `Polybase` will log and discard the duplicate proposal.

### Leader Election

Leader election is done for every proposal/accept round (accounting for timeouts) using the following simple formula:

```
 next_leader_id = skips % number_of_peers 

```

So this is **not** a Round-Robin procedure (as in some consensus protocols), but rather depends on the number of current peers (which is fixed at start-up for now), and the number of skips (i.e, since the last confirmed proposal) thus far. 


## Todo
 
 - Add/remove peers from the protocol validators
