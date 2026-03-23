# Introduction

Blockchain technology has revolutionised the digital realm, enabling an era of decentralisation, security, and trust without intermediaries. These benefits have spurred many innovations across diverse sectors, enabling more transparent, immutable, and efficient transactions. Yet, for all their transformative advantages, blockchains have encountered a significant hurdle: privacy. This limitation has, in many ways, restrained the true potential of blockchains, especially in sectors where confidentiality is paramount, such as traditional finance, real world assets and payments.

Historically, achieving both transparency and privacy in blockchains appeared mutually exclusive. However, with the emergence of Zero-Knowledge (ZK) proofs, a form of cryptography for provable computation in zero knowledge domains, it has become feasible to ensure transaction privacy without compromising the integrity that blockchain platforms promise.

A side effect of enabling privacy, however, is that it complicates regulatory compliance. With public blockchains, the natural transparency enables regulators to screen all transactions for potential compliance breaches. With privacy, everything becomes more opaque and it becomes significantly more difficult to monitor transactions effectively. Yet this too, can be solved through zero knowledge, with clients generating relevant proofs that their activity has been compliant, so called “Proof of Innocence”.

In addition, Layer 2 (L2) solutions have taken center stage in the quest to address blockchain scalability, reducing the strain on the main chain and ensuring faster, more efficient operations. By combining the benefits of L2 with the privacy and compliance capabilities of ZK proofs, a new paradigm emerges.

In this paper, we present a full end to end solution for providing on chain privacy and compliance via an Ethereum L2 rollup, powered by zero knowledge.
