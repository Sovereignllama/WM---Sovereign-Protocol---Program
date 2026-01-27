/**
 * Program IDL in camelCase format in order to be used in JS/TS.
 *
 * Note that this is only a type helper and is not the actual IDL. The original
 * IDL can be found at `target/idl/sovereign_liquidity.json`.
 */
export type SovereignLiquidity = {
  "address": "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",
  "metadata": {
    "name": "sovereignLiquidity",
    "version": "0.1.0",
    "spec": "0.1.0",
    "description": "Sovereign Liquidity Protocol - Fair launch with recovery-first LP and Genesis NFT fee rights"
  },
  "instructions": [
    {
      "name": "cancelActivityCheck",
      "docs": [
        "Creator cancels activity check (proves liveness)"
      ],
      "discriminator": [
        34,
        221,
        236,
        21,
        33,
        119,
        155,
        132
      ],
      "accounts": [
        {
          "name": "creator",
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "claimDepositorFees",
      "docs": [
        "Claim depositor's share of accumulated fees"
      ],
      "discriminator": [
        25,
        69,
        126,
        212,
        100,
        100,
        130,
        211
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "feeVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "claimFees",
      "docs": [
        "Collect fees from Whirlpool position"
      ],
      "discriminator": [
        82,
        251,
        233,
        156,
        12,
        52,
        184,
        202
      ],
      "accounts": [
        {
          "name": "claimer",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "permanentLock",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  101,
                  114,
                  109,
                  97,
                  110,
                  101,
                  110,
                  116,
                  95,
                  108,
                  111,
                  99,
                  107
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "position",
          "writable": true
        },
        {
          "name": "tokenVaultA",
          "writable": true
        },
        {
          "name": "tokenVaultB",
          "writable": true
        },
        {
          "name": "feeVault",
          "docs": [
            "Fee destination for SOL fees"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "creatorFeeTracker",
          "docs": [
            "Creator fee tracker"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  111,
                  114,
                  95,
                  116,
                  114,
                  97,
                  99,
                  107,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "whirlpoolProgram",
          "address": "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "claimUnwind",
      "docs": [
        "Claim proceeds from unwound sovereign"
      ],
      "discriminator": [
        196,
        153,
        9,
        184,
        35,
        38,
        200,
        17
      ],
      "accounts": [
        {
          "name": "claimer",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "claimer"
              }
            ]
          }
        },
        {
          "name": "solVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenVault",
          "docs": [
            "Token vault for token distribution"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "claimerTokenAccount",
          "docs": [
            "Claimer's token account"
          ],
          "writable": true
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "createSovereign",
      "docs": [
        "Create a new sovereign (token launch or BYO token)"
      ],
      "discriminator": [
        173,
        247,
        75,
        229,
        177,
        225,
        13,
        222
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "docs": [
            "Use Box to reduce stack usage - ProtocolState is a large account"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "sovereign",
          "docs": [
            "Use Box to reduce stack usage - SovereignState is the largest account"
          ],
          "writable": true
        },
        {
          "name": "creatorTracker",
          "docs": [
            "Use Box to reduce stack usage"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  111,
                  114,
                  95,
                  116,
                  114,
                  97,
                  99,
                  107,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "creationFeeEscrow",
          "docs": [
            "Use Box to reduce stack usage"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  105,
                  111,
                  110,
                  95,
                  102,
                  101,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "Token mint - for BYO this is the existing token",
            "For TokenLaunch this will be created in a separate instruction"
          ],
          "optional": true
        },
        {
          "name": "creatorTokenAccount",
          "docs": [
            "Creator's token account (for BYO token transfer)"
          ],
          "writable": true,
          "optional": true
        },
        {
          "name": "tokenVault",
          "docs": [
            "Sovereign's token vault - Use Box to reduce stack usage"
          ],
          "writable": true,
          "optional": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "params",
          "type": {
            "defined": {
              "name": "createSovereignParams"
            }
          }
        }
      ]
    },
    {
      "name": "deposit",
      "docs": [
        "Deposit SOL during bonding phase"
      ],
      "discriminator": [
        242,
        35,
        198,
        137,
        82,
        225,
        242,
        182
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "docs": [
            "Deposit record - initialized if new depositor"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "solVault",
          "docs": [
            "SOL vault to hold deposits during bonding"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "executeActivityCheck",
      "docs": [
        "Execute activity check after 90 days"
      ],
      "discriminator": [
        91,
        36,
        35,
        187,
        40,
        180,
        22,
        216
      ],
      "accounts": [
        {
          "name": "executor",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "executeUnwind",
      "docs": [
        "Execute unwind after proposal passes"
      ],
      "discriminator": [
        245,
        215,
        31,
        118,
        91,
        202,
        247,
        32
      ],
      "accounts": [
        {
          "name": "executor",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "proposal",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  112,
                  111,
                  115,
                  97,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "proposal.proposal_id",
                "account": "proposal"
              }
            ]
          }
        },
        {
          "name": "permanentLock",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  101,
                  114,
                  109,
                  97,
                  110,
                  101,
                  110,
                  116,
                  95,
                  108,
                  111,
                  99,
                  107
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "Token mint"
          ],
          "writable": true
        },
        {
          "name": "position",
          "writable": true
        },
        {
          "name": "whirlpoolProgram",
          "address": "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        },
        {
          "name": "solVault",
          "docs": [
            "Vault to receive removed liquidity"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "finalize",
      "docs": [
        "Finalize sovereign after bond target is met",
        "Creates Whirlpool, adds liquidity, mints NFTs"
      ],
      "discriminator": [
        171,
        61,
        218,
        56,
        127,
        115,
        12,
        217
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "tokenMint",
          "docs": [
            "Token mint for the sovereign token"
          ],
          "writable": true
        },
        {
          "name": "solVault",
          "docs": [
            "SOL vault holding deposits"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "tokenVault",
          "docs": [
            "Token vault to hold LP tokens"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  116,
                  111,
                  107,
                  101,
                  110,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "permanentLock",
          "docs": [
            "Permanent lock account"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  101,
                  114,
                  109,
                  97,
                  110,
                  101,
                  110,
                  116,
                  95,
                  108,
                  111,
                  99,
                  107
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "nftCollectionMint",
          "docs": [
            "Genesis NFT collection mint (created during sovereign creation)"
          ],
          "writable": true
        },
        {
          "name": "whirlpoolProgram",
          "address": "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "finalizeVote",
      "docs": [
        "Finalize voting after period ends"
      ],
      "discriminator": [
        181,
        176,
        6,
        248,
        249,
        134,
        146,
        56
      ],
      "accounts": [
        {
          "name": "caller",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "proposal",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  112,
                  111,
                  115,
                  97,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "proposal.proposal_id",
                "account": "proposal"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "initializeProtocol",
      "docs": [
        "Initialize the protocol state (one-time setup)"
      ],
      "discriminator": [
        188,
        233,
        252,
        106,
        134,
        146,
        202,
        91
      ],
      "accounts": [
        {
          "name": "authority",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "treasury"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "initiateActivityCheck",
      "docs": [
        "Initiate activity check (90-day countdown)"
      ],
      "discriminator": [
        133,
        15,
        165,
        86,
        217,
        160,
        30,
        25
      ],
      "accounts": [
        {
          "name": "initiator",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "markBondingFailed",
      "docs": [
        "Mark bonding as failed if deadline passed"
      ],
      "discriminator": [
        90,
        22,
        152,
        88,
        54,
        82,
        80,
        230
      ],
      "accounts": [
        {
          "name": "caller",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "mintGenesisNft",
      "docs": [
        "Mint Genesis NFT to a depositor after finalization"
      ],
      "discriminator": [
        188,
        218,
        134,
        12,
        220,
        55,
        97,
        254
      ],
      "accounts": [
        {
          "name": "payer",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "depositor"
        },
        {
          "name": "nftMint",
          "docs": [
            "Genesis NFT mint for this specific depositor"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  103,
                  101,
                  110,
                  101,
                  115,
                  105,
                  115,
                  95,
                  110,
                  102,
                  116,
                  95,
                  109,
                  105,
                  110,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "nftTokenAccount",
          "docs": [
            "NFT token account for the depositor"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "account",
                "path": "depositor"
              },
              {
                "kind": "const",
                "value": [
                  6,
                  221,
                  246,
                  225,
                  215,
                  101,
                  161,
                  147,
                  217,
                  203,
                  225,
                  70,
                  206,
                  235,
                  121,
                  172,
                  28,
                  180,
                  133,
                  237,
                  95,
                  91,
                  55,
                  145,
                  58,
                  140,
                  245,
                  133,
                  126,
                  255,
                  0,
                  169
                ]
              },
              {
                "kind": "account",
                "path": "nftMint"
              }
            ],
            "program": {
              "kind": "const",
              "value": [
                140,
                151,
                37,
                143,
                78,
                36,
                137,
                241,
                187,
                61,
                16,
                41,
                20,
                142,
                13,
                131,
                11,
                90,
                19,
                153,
                218,
                255,
                16,
                132,
                4,
                142,
                123,
                216,
                219,
                233,
                248,
                89
              ]
            }
          }
        },
        {
          "name": "metadataAccount",
          "writable": true
        },
        {
          "name": "metadataProgram",
          "address": "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
        },
        {
          "name": "tokenProgram",
          "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        },
        {
          "name": "associatedTokenProgram",
          "address": "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        },
        {
          "name": "rent",
          "address": "SysvarRent111111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "proposeUnwind",
      "docs": [
        "Propose to unwind the sovereign (Genesis NFT holders)"
      ],
      "discriminator": [
        72,
        168,
        80,
        19,
        92,
        71,
        236,
        127
      ],
      "accounts": [
        {
          "name": "proposer",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "docs": [
            "Proposer must hold Genesis NFT"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "proposer"
              }
            ]
          }
        },
        {
          "name": "proposal",
          "docs": [
            "Proposal account"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  112,
                  111,
                  115,
                  97,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "sovereign.proposal_count",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "renounceFeeThreshold",
      "docs": [
        "Permanently renounce fee threshold (irreversible)"
      ],
      "discriminator": [
        235,
        172,
        32,
        20,
        123,
        134,
        216,
        147
      ],
      "accounts": [
        {
          "name": "creator",
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "creatorFeeTracker",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  111,
                  114,
                  95,
                  116,
                  114,
                  97,
                  99,
                  107,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "setProtocolPaused",
      "docs": [
        "Pause/unpause protocol (emergency)"
      ],
      "discriminator": [
        47,
        62,
        75,
        69,
        166,
        0,
        147,
        157
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "protocolState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "paused",
          "type": "bool"
        }
      ]
    },
    {
      "name": "transferProtocolAuthority",
      "docs": [
        "Transfer protocol authority"
      ],
      "discriminator": [
        35,
        76,
        36,
        77,
        136,
        112,
        158,
        222
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "newAuthority"
        },
        {
          "name": "protocolState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": []
    },
    {
      "name": "updateFeeThreshold",
      "docs": [
        "Update creator's fee threshold (can only decrease)"
      ],
      "discriminator": [
        78,
        81,
        212,
        95,
        133,
        180,
        7,
        137
      ],
      "accounts": [
        {
          "name": "creator",
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "creatorFeeTracker",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  111,
                  114,
                  95,
                  116,
                  114,
                  97,
                  99,
                  107,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "newThresholdBps",
          "type": "u16"
        }
      ]
    },
    {
      "name": "updateProtocolFees",
      "docs": [
        "Update protocol fee parameters"
      ],
      "discriminator": [
        158,
        219,
        253,
        143,
        54,
        45,
        113,
        182
      ],
      "accounts": [
        {
          "name": "authority",
          "signer": true
        },
        {
          "name": "protocolState",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        }
      ],
      "args": [
        {
          "name": "newCreationFeeBps",
          "type": {
            "option": "u16"
          }
        },
        {
          "name": "newMinFeeLamports",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "newMinDeposit",
          "type": {
            "option": "u64"
          }
        },
        {
          "name": "newMinBondTarget",
          "type": {
            "option": "u64"
          }
        }
      ]
    },
    {
      "name": "vote",
      "docs": [
        "Vote on an unwind proposal"
      ],
      "discriminator": [
        227,
        110,
        155,
        23,
        136,
        126,
        172,
        25
      ],
      "accounts": [
        {
          "name": "voter",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "docs": [
            "Voter's deposit record with NFT"
          ],
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "voter"
              }
            ]
          }
        },
        {
          "name": "proposal",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  112,
                  111,
                  115,
                  97,
                  108
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "proposal.proposal_id",
                "account": "proposal"
              }
            ]
          }
        },
        {
          "name": "voteRecord",
          "docs": [
            "Vote record - tracks individual votes"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  118,
                  111,
                  116,
                  101,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "proposal"
              },
              {
                "kind": "account",
                "path": "voter"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "support",
          "type": "bool"
        }
      ]
    },
    {
      "name": "withdraw",
      "docs": [
        "Withdraw SOL during bonding phase (investors only)"
      ],
      "discriminator": [
        183,
        18,
        70,
        156,
        148,
        109,
        161,
        34
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "protocolState",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  112,
                  114,
                  111,
                  116,
                  111,
                  99,
                  111,
                  108,
                  95,
                  115,
                  116,
                  97,
                  116,
                  101
                ]
              }
            ]
          }
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "solVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": [
        {
          "name": "amount",
          "type": "u64"
        }
      ]
    },
    {
      "name": "withdrawCreatorFailed",
      "docs": [
        "Creator withdraws escrow from failed bonding"
      ],
      "discriminator": [
        177,
        175,
        108,
        205,
        111,
        166,
        48,
        132
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "solVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "creationFeeEscrow",
          "docs": [
            "Creation fee escrow - returned to creator on failure"
          ],
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  105,
                  111,
                  110,
                  95,
                  102,
                  101,
                  101,
                  95,
                  101,
                  115,
                  99,
                  114,
                  111,
                  119
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "withdrawCreatorFees",
      "docs": [
        "Creator withdraws earned fees"
      ],
      "discriminator": [
        8,
        30,
        213,
        18,
        121,
        105,
        129,
        222
      ],
      "accounts": [
        {
          "name": "creator",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "creatorFeeTracker",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  99,
                  114,
                  101,
                  97,
                  116,
                  111,
                  114,
                  95,
                  116,
                  114,
                  97,
                  99,
                  107,
                  101,
                  114
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "feeVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    },
    {
      "name": "withdrawFailed",
      "docs": [
        "Investor withdraws from failed bonding"
      ],
      "discriminator": [
        235,
        53,
        122,
        63,
        87,
        220,
        130,
        215
      ],
      "accounts": [
        {
          "name": "depositor",
          "writable": true,
          "signer": true
        },
        {
          "name": "sovereign",
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  118,
                  101,
                  114,
                  101,
                  105,
                  103,
                  110
                ]
              },
              {
                "kind": "account",
                "path": "sovereign.sovereign_id",
                "account": "sovereignState"
              }
            ]
          }
        },
        {
          "name": "depositRecord",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  100,
                  101,
                  112,
                  111,
                  115,
                  105,
                  116,
                  95,
                  114,
                  101,
                  99,
                  111,
                  114,
                  100
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              },
              {
                "kind": "account",
                "path": "depositor"
              }
            ]
          }
        },
        {
          "name": "solVault",
          "writable": true,
          "pda": {
            "seeds": [
              {
                "kind": "const",
                "value": [
                  115,
                  111,
                  108,
                  95,
                  118,
                  97,
                  117,
                  108,
                  116
                ]
              },
              {
                "kind": "account",
                "path": "sovereign"
              }
            ]
          }
        },
        {
          "name": "systemProgram",
          "address": "11111111111111111111111111111111"
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "creationFeeEscrow",
      "discriminator": [
        233,
        63,
        61,
        182,
        5,
        103,
        109,
        17
      ]
    },
    {
      "name": "creatorFeeTracker",
      "discriminator": [
        144,
        62,
        251,
        103,
        232,
        2,
        131,
        177
      ]
    },
    {
      "name": "depositRecord",
      "discriminator": [
        83,
        232,
        10,
        31,
        251,
        49,
        189,
        167
      ]
    },
    {
      "name": "permanentLock",
      "discriminator": [
        82,
        237,
        74,
        160,
        95,
        15,
        144,
        180
      ]
    },
    {
      "name": "proposal",
      "discriminator": [
        26,
        94,
        189,
        187,
        116,
        136,
        53,
        33
      ]
    },
    {
      "name": "protocolState",
      "discriminator": [
        33,
        51,
        173,
        134,
        35,
        140,
        195,
        248
      ]
    },
    {
      "name": "sovereignState",
      "discriminator": [
        42,
        162,
        40,
        206,
        227,
        8,
        23,
        212
      ]
    },
    {
      "name": "voteRecord",
      "discriminator": [
        112,
        9,
        123,
        165,
        234,
        9,
        157,
        167
      ]
    }
  ],
  "events": [
    {
      "name": "activityCheckExecuted",
      "discriminator": [
        136,
        51,
        54,
        218,
        40,
        224,
        168,
        118
      ]
    },
    {
      "name": "activityCheckInitiated",
      "discriminator": [
        210,
        119,
        56,
        139,
        58,
        205,
        209,
        215
      ]
    },
    {
      "name": "bondingFailed",
      "discriminator": [
        252,
        251,
        221,
        232,
        244,
        209,
        15,
        53
      ]
    },
    {
      "name": "creatorEscrowed",
      "discriminator": [
        36,
        170,
        128,
        250,
        50,
        101,
        251,
        139
      ]
    },
    {
      "name": "creatorFailedWithdrawal",
      "discriminator": [
        111,
        25,
        187,
        111,
        199,
        230,
        62,
        62
      ]
    },
    {
      "name": "creatorMarketBuyExecuted",
      "discriminator": [
        20,
        88,
        173,
        10,
        82,
        209,
        183,
        46
      ]
    },
    {
      "name": "failedWithdrawal",
      "discriminator": [
        123,
        179,
        129,
        126,
        250,
        130,
        70,
        107
      ]
    },
    {
      "name": "feeThresholdRenounced",
      "discriminator": [
        53,
        165,
        108,
        68,
        229,
        41,
        1,
        62
      ]
    },
    {
      "name": "feeThresholdUpdated",
      "discriminator": [
        44,
        213,
        134,
        129,
        248,
        94,
        27,
        145
      ]
    },
    {
      "name": "feesClaimed",
      "discriminator": [
        22,
        104,
        110,
        222,
        38,
        157,
        14,
        62
      ]
    },
    {
      "name": "genesisNftMinted",
      "discriminator": [
        195,
        85,
        73,
        18,
        12,
        205,
        137,
        16
      ]
    },
    {
      "name": "investorDeposited",
      "discriminator": [
        167,
        232,
        74,
        118,
        168,
        48,
        249,
        170
      ]
    },
    {
      "name": "investorWithdrew",
      "discriminator": [
        253,
        253,
        70,
        118,
        149,
        54,
        114,
        18
      ]
    },
    {
      "name": "poolRestricted",
      "discriminator": [
        19,
        188,
        177,
        232,
        218,
        172,
        177,
        242
      ]
    },
    {
      "name": "proposalCreated",
      "discriminator": [
        186,
        8,
        160,
        108,
        81,
        13,
        51,
        206
      ]
    },
    {
      "name": "proposalFinalized",
      "discriminator": [
        159,
        104,
        210,
        220,
        86,
        209,
        61,
        51
      ]
    },
    {
      "name": "protocolFeesUpdated",
      "discriminator": [
        190,
        127,
        198,
        224,
        14,
        253,
        180,
        26
      ]
    },
    {
      "name": "protocolInitialized",
      "discriminator": [
        173,
        122,
        168,
        254,
        9,
        118,
        76,
        132
      ]
    },
    {
      "name": "recoveryComplete",
      "discriminator": [
        125,
        91,
        195,
        23,
        239,
        223,
        59,
        133
      ]
    },
    {
      "name": "sovereignCreated",
      "discriminator": [
        201,
        212,
        126,
        161,
        113,
        210,
        185,
        208
      ]
    },
    {
      "name": "sovereignFinalized",
      "discriminator": [
        74,
        31,
        70,
        10,
        174,
        200,
        37,
        179
      ]
    },
    {
      "name": "unwindClaimed",
      "discriminator": [
        27,
        150,
        94,
        107,
        97,
        155,
        0,
        251
      ]
    },
    {
      "name": "unwindExecuted",
      "discriminator": [
        206,
        6,
        243,
        239,
        134,
        228,
        10,
        39
      ]
    },
    {
      "name": "voteCast",
      "discriminator": [
        39,
        53,
        195,
        104,
        188,
        17,
        225,
        213
      ]
    }
  ],
  "errors": [
    {
      "code": 6000,
      "name": "invalidState",
      "msg": "Sovereign is not in the expected state"
    },
    {
      "code": 6001,
      "name": "deadlinePassed",
      "msg": "Bonding deadline has passed"
    },
    {
      "code": 6002,
      "name": "deadlineNotPassed",
      "msg": "Bonding deadline has not passed yet"
    },
    {
      "code": 6003,
      "name": "bondingNotComplete",
      "msg": "Bonding target not yet met"
    },
    {
      "code": 6004,
      "name": "bondingComplete",
      "msg": "Bonding target already met"
    },
    {
      "code": 6005,
      "name": "recoveryNotComplete",
      "msg": "Recovery phase is not complete"
    },
    {
      "code": 6006,
      "name": "recoveryAlreadyComplete",
      "msg": "Recovery phase is already complete"
    },
    {
      "code": 6007,
      "name": "creatorDepositExceedsMax",
      "msg": "Creator deposit exceeds maximum allowed (1% of bond target)"
    },
    {
      "code": 6008,
      "name": "zeroDeposit",
      "msg": "Deposit amount is zero"
    },
    {
      "code": 6009,
      "name": "depositTooSmall",
      "msg": "Deposit amount below minimum (0.1 SOL)"
    },
    {
      "code": 6010,
      "name": "noDepositRecord",
      "msg": "No deposit record found"
    },
    {
      "code": 6011,
      "name": "depositExceedsBondTarget",
      "msg": "Deposit exceeds bond target"
    },
    {
      "code": 6012,
      "name": "insufficientDeposit",
      "msg": "Withdrawal amount exceeds deposit"
    },
    {
      "code": 6013,
      "name": "zeroWithdraw",
      "msg": "Withdrawal amount is zero"
    },
    {
      "code": 6014,
      "name": "insufficientDepositBalance",
      "msg": "Insufficient deposit balance"
    },
    {
      "code": 6015,
      "name": "creatorCannotWithdrawDuringBonding",
      "msg": "Creator cannot withdraw during bonding phase"
    },
    {
      "code": 6016,
      "name": "insufficientVaultBalance",
      "msg": "Insufficient vault balance"
    },
    {
      "code": 6017,
      "name": "nothingToWithdraw",
      "msg": "Nothing to withdraw"
    },
    {
      "code": 6018,
      "name": "creatorMustUseCreatorWithdraw",
      "msg": "Creator must use creator-specific withdraw instruction"
    },
    {
      "code": 6019,
      "name": "notNftOwner",
      "msg": "Caller is not the NFT owner"
    },
    {
      "code": 6020,
      "name": "nftAlreadyUsed",
      "msg": "NFT has already been used for this action"
    },
    {
      "code": 6021,
      "name": "nftAlreadyMinted",
      "msg": "NFT has already been minted"
    },
    {
      "code": 6022,
      "name": "wrongNft",
      "msg": "Wrong NFT for this deposit record"
    },
    {
      "code": 6023,
      "name": "nftNotMinted",
      "msg": "NFT not yet minted"
    },
    {
      "code": 6024,
      "name": "noGenesisNft",
      "msg": "No Genesis NFT - cannot participate in governance"
    },
    {
      "code": 6025,
      "name": "creatorCannotClaimDuringRecovery",
      "msg": "Creator cannot claim fees during recovery phase"
    },
    {
      "code": 6026,
      "name": "creatorCannotVote",
      "msg": "Creator cannot vote during recovery phase"
    },
    {
      "code": 6027,
      "name": "creatorTokensLocked",
      "msg": "Creator tokens are locked until recovery complete or unwind"
    },
    {
      "code": 6028,
      "name": "insufficientInactivity",
      "msg": "Not enough inactivity to propose unwind"
    },
    {
      "code": 6029,
      "name": "votingNotEnded",
      "msg": "Voting period has not ended"
    },
    {
      "code": 6030,
      "name": "votingPeriodNotEnded",
      "msg": "Voting period has not ended yet"
    },
    {
      "code": 6031,
      "name": "votingEnded",
      "msg": "Voting period has ended"
    },
    {
      "code": 6032,
      "name": "votingPeriodEnded",
      "msg": "Voting period has ended"
    },
    {
      "code": 6033,
      "name": "quorumNotReached",
      "msg": "Proposal did not reach quorum (67%)"
    },
    {
      "code": 6034,
      "name": "proposalNotPassed",
      "msg": "Proposal did not pass (need 51%)"
    },
    {
      "code": 6035,
      "name": "proposalNotActive",
      "msg": "Proposal is not active"
    },
    {
      "code": 6036,
      "name": "alreadyVoted",
      "msg": "Already voted on this proposal"
    },
    {
      "code": 6037,
      "name": "governanceNotActive",
      "msg": "Governance is only active during recovery phase"
    },
    {
      "code": 6038,
      "name": "noVotingPower",
      "msg": "No voting power"
    },
    {
      "code": 6039,
      "name": "timelockNotExpired",
      "msg": "Timelock period has not expired"
    },
    {
      "code": 6040,
      "name": "proposalAlreadyExecuted",
      "msg": "Proposal already executed"
    },
    {
      "code": 6041,
      "name": "activeProposalExists",
      "msg": "Active proposal already exists"
    },
    {
      "code": 6042,
      "name": "cannotGovernanceUnwindInActivePhase",
      "msg": "Cannot unwind in active phase via governance"
    },
    {
      "code": 6043,
      "name": "autoUnwindConditionsNotMet",
      "msg": "Auto-unwind conditions not met"
    },
    {
      "code": 6044,
      "name": "onlyActivePhase",
      "msg": "Activity check only valid in Active phase"
    },
    {
      "code": 6045,
      "name": "activityCheckAlreadyInProgress",
      "msg": "Activity check already in progress"
    },
    {
      "code": 6046,
      "name": "activityCheckAlreadyPending",
      "msg": "Activity check already pending"
    },
    {
      "code": 6047,
      "name": "noActivityCheckInProgress",
      "msg": "No activity check in progress"
    },
    {
      "code": 6048,
      "name": "noActivityCheckPending",
      "msg": "No activity check pending"
    },
    {
      "code": 6049,
      "name": "activityCheckTooEarly",
      "msg": "Must wait 90+ days before executing activity check"
    },
    {
      "code": 6050,
      "name": "activityCheckPeriodNotElapsed",
      "msg": "Activity check period has not elapsed"
    },
    {
      "code": 6051,
      "name": "activityCheckCooldownNotExpired",
      "msg": "Must wait 7 days after cancelled check before initiating new one"
    },
    {
      "code": 6052,
      "name": "feeThresholdRenounced",
      "msg": "Fee threshold has been renounced and cannot be changed"
    },
    {
      "code": 6053,
      "name": "alreadyRenounced",
      "msg": "Fee threshold already renounced"
    },
    {
      "code": 6054,
      "name": "feeThresholdAlreadyRenounced",
      "msg": "Fee threshold already renounced"
    },
    {
      "code": 6055,
      "name": "cannotIncreaseFeeThreshold",
      "msg": "Cannot increase fee threshold"
    },
    {
      "code": 6056,
      "name": "invalidFeeThreshold",
      "msg": "Invalid fee threshold"
    },
    {
      "code": 6057,
      "name": "invalidPool",
      "msg": "Invalid pool - does not match sovereign's whirlpool"
    },
    {
      "code": 6058,
      "name": "invalidProgram",
      "msg": "Invalid program ID for CPI"
    },
    {
      "code": 6059,
      "name": "invalidTreasury",
      "msg": "Invalid treasury address - cannot be zero"
    },
    {
      "code": 6060,
      "name": "invalidBondTarget",
      "msg": "Invalid bond target - must be at least 50 SOL"
    },
    {
      "code": 6061,
      "name": "invalidBondDuration",
      "msg": "Invalid bond duration - must be 7-30 days"
    },
    {
      "code": 6062,
      "name": "invalidSellFee",
      "msg": "Invalid sell fee - must be 0-3%"
    },
    {
      "code": 6063,
      "name": "invalidAmount",
      "msg": "Invalid amount"
    },
    {
      "code": 6064,
      "name": "bondTargetNotMet",
      "msg": "Bond target not met"
    },
    {
      "code": 6065,
      "name": "bondTargetMet",
      "msg": "Bond target already met"
    },
    {
      "code": 6066,
      "name": "unauthorized",
      "msg": "unauthorized"
    },
    {
      "code": 6067,
      "name": "feeTooHigh",
      "msg": "Fee too high"
    },
    {
      "code": 6068,
      "name": "poolRestricted",
      "msg": "Pool is restricted - only Genesis position can LP"
    },
    {
      "code": 6069,
      "name": "poolNotRestricted",
      "msg": "Pool is not restricted"
    },
    {
      "code": 6070,
      "name": "positionAlreadyUnwound",
      "msg": "Position already unwound"
    },
    {
      "code": 6071,
      "name": "invalidPosition",
      "msg": "Invalid position - does not match permanent lock"
    },
    {
      "code": 6072,
      "name": "sellFeeExceedsMax",
      "msg": "Sell fee exceeds maximum (3%)"
    },
    {
      "code": 6073,
      "name": "creationFeeExceedsMax",
      "msg": "Creation fee exceeds maximum (10%)"
    },
    {
      "code": 6074,
      "name": "unwindFeeExceedsMax",
      "msg": "Unwind fee exceeds maximum (10%)"
    },
    {
      "code": 6075,
      "name": "feeControlRenounced",
      "msg": "Fee control has been renounced"
    },
    {
      "code": 6076,
      "name": "insufficientCreationFee",
      "msg": "Insufficient creation fee"
    },
    {
      "code": 6077,
      "name": "notProtocolAuthority",
      "msg": "Caller is not the protocol authority"
    },
    {
      "code": 6078,
      "name": "invalidAutoUnwindPeriod",
      "msg": "Auto-unwind period outside valid range (90-365 days)"
    },
    {
      "code": 6079,
      "name": "metadataUriTooLong",
      "msg": "Token metadata URI is too long"
    },
    {
      "code": 6080,
      "name": "tokenNameTooLong",
      "msg": "Token name is too long"
    },
    {
      "code": 6081,
      "name": "tokenSymbolTooLong",
      "msg": "Token symbol is too long"
    },
    {
      "code": 6082,
      "name": "missingTokenName",
      "msg": "Token Launcher: Missing token name"
    },
    {
      "code": 6083,
      "name": "missingTokenSymbol",
      "msg": "Token Launcher: Missing token symbol"
    },
    {
      "code": 6084,
      "name": "missingTokenSupply",
      "msg": "Token Launcher: Missing token supply"
    },
    {
      "code": 6085,
      "name": "missingExistingMint",
      "msg": "BYO Token: Missing existing mint address"
    },
    {
      "code": 6086,
      "name": "missingDepositAmount",
      "msg": "BYO Token: Missing deposit amount"
    },
    {
      "code": 6087,
      "name": "insufficientTokenDeposit",
      "msg": "BYO Token: Insufficient token deposit (below minimum % required)"
    },
    {
      "code": 6088,
      "name": "failedToReadTokenSupply",
      "msg": "BYO Token: Failed to read token supply"
    },
    {
      "code": 6089,
      "name": "alreadyClaimed",
      "msg": "Already claimed"
    },
    {
      "code": 6090,
      "name": "nothingToClaim",
      "msg": "Nothing to claim"
    },
    {
      "code": 6091,
      "name": "notCreator",
      "msg": "Caller is not the creator"
    },
    {
      "code": 6092,
      "name": "notDepositor",
      "msg": "Caller is not the depositor"
    },
    {
      "code": 6093,
      "name": "overflow",
      "msg": "Arithmetic overflow"
    },
    {
      "code": 6094,
      "name": "underflow",
      "msg": "Arithmetic underflow"
    },
    {
      "code": 6095,
      "name": "divisionByZero",
      "msg": "Division by zero"
    },
    {
      "code": 6096,
      "name": "noDeposits",
      "msg": "No deposits in the sovereign"
    },
    {
      "code": 6097,
      "name": "slippageExceeded",
      "msg": "Slippage tolerance exceeded"
    },
    {
      "code": 6098,
      "name": "protocolPaused",
      "msg": "Protocol is currently paused"
    },
    {
      "code": 6099,
      "name": "activityCheckCooldownNotElapsed",
      "msg": "Activity check cooldown has not elapsed (7 days required)"
    }
  ],
  "types": [
    {
      "name": "activityCheckExecuted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "executor",
            "type": "pubkey"
          },
          {
            "name": "executedAt",
            "type": "i64"
          },
          {
            "name": "daysElapsed",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "activityCheckInitiated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "initiator",
            "type": "pubkey"
          },
          {
            "name": "initiatedAt",
            "type": "i64"
          },
          {
            "name": "executionAvailableAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "bondingFailed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "bondTarget",
            "type": "u64"
          },
          {
            "name": "failedAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "createSovereignParams",
      "docs": [
        "Parameters for creating a new sovereign"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignType",
            "docs": [
              "Type of launch (TokenLaunch or BYOToken)"
            ],
            "type": {
              "defined": {
                "name": "sovereignType"
              }
            }
          },
          {
            "name": "bondTarget",
            "docs": [
              "SOL to raise (in lamports)"
            ],
            "type": "u64"
          },
          {
            "name": "bondDuration",
            "docs": [
              "Duration in seconds (7-30 days)"
            ],
            "type": "i64"
          },
          {
            "name": "name",
            "docs": [
              "Sovereign name (for metadata)"
            ],
            "type": "string"
          },
          {
            "name": "tokenName",
            "type": {
              "option": "string"
            }
          },
          {
            "name": "tokenSymbol",
            "type": {
              "option": "string"
            }
          },
          {
            "name": "tokenSupply",
            "type": {
              "option": "u64"
            }
          },
          {
            "name": "sellFeeBps",
            "type": {
              "option": "u16"
            }
          },
          {
            "name": "feeMode",
            "type": {
              "option": {
                "defined": {
                  "name": "feeMode"
                }
              }
            }
          },
          {
            "name": "metadataUri",
            "type": {
              "option": "string"
            }
          },
          {
            "name": "depositAmount",
            "type": {
              "option": "u64"
            }
          }
        ]
      }
    },
    {
      "name": "creationFeeEscrow",
      "docs": [
        "Escrow account for holding creation fee during bonding"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereign",
            "docs": [
              "The sovereign this escrow belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount escrowed in lamports"
            ],
            "type": "u64"
          },
          {
            "name": "released",
            "docs": [
              "Whether fee has been released (to treasury or refunded)"
            ],
            "type": "bool"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "creatorEscrowed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "totalEscrowed",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "creatorFailedWithdrawal",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "escrowReturned",
            "type": "u64"
          },
          {
            "name": "creationFeeReturned",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "creatorFeeTracker",
      "docs": [
        "Tracks creator's fee revenue and purchased tokens",
        "Separate from DepositRecord because creator has different rules:",
        "- Creator deposits TOKENS (not SOL to LP)",
        "- Creator's SOL is escrowed for market buy",
        "- Creator does NOT get Genesis NFT",
        "- Creator does NOT get LP fee share"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereign",
            "docs": [
              "The sovereign this tracker belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "creator",
            "docs": [
              "The creator's wallet address"
            ],
            "type": "pubkey"
          },
          {
            "name": "totalEarned",
            "docs": [
              "Total fees earned by creator"
            ],
            "type": "u64"
          },
          {
            "name": "totalClaimed",
            "docs": [
              "Total fees claimed by creator"
            ],
            "type": "u64"
          },
          {
            "name": "pendingWithdrawal",
            "docs": [
              "Pending withdrawal amount"
            ],
            "type": "u64"
          },
          {
            "name": "thresholdRenounced",
            "docs": [
              "Whether fee threshold has been renounced"
            ],
            "type": "bool"
          },
          {
            "name": "purchasedTokens",
            "docs": [
              "Tokens purchased via market buy (from escrowed SOL)"
            ],
            "type": "u64"
          },
          {
            "name": "tokensLocked",
            "docs": [
              "Whether purchased tokens are locked"
            ],
            "type": "bool"
          },
          {
            "name": "purchasedTokensClaimed",
            "docs": [
              "Whether creator has claimed purchased tokens (after recovery)"
            ],
            "type": "bool"
          },
          {
            "name": "tokensClaimed",
            "docs": [
              "Whether creator has claimed unwind tokens (LP tokens + purchased)"
            ],
            "type": "bool"
          },
          {
            "name": "sellTaxAccumulated",
            "docs": [
              "Sell tax revenue accumulated (Token Launcher only)"
            ],
            "type": "u64"
          },
          {
            "name": "sellTaxClaimed",
            "docs": [
              "Sell tax revenue claimed by creator"
            ],
            "type": "u64"
          },
          {
            "name": "failedReclaimed",
            "docs": [
              "Whether creator has reclaimed on failed bonding"
            ],
            "type": "bool"
          },
          {
            "name": "purchasedAt",
            "docs": [
              "Timestamp when tokens were purchased"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "creatorMarketBuyExecuted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "solAmount",
            "type": "u64"
          },
          {
            "name": "tokensReceived",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "depositRecord",
      "docs": [
        "Tracks an investor's deposit in a sovereign",
        "One DepositRecord per depositor per sovereign",
        "NOTE: Creator does NOT have a DepositRecord (they use CreatorFeeTracker)"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereign",
            "docs": [
              "The sovereign this deposit belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "depositor",
            "docs": [
              "The investor's wallet address"
            ],
            "type": "pubkey"
          },
          {
            "name": "amount",
            "docs": [
              "Amount deposited in lamports"
            ],
            "type": "u64"
          },
          {
            "name": "sharesBps",
            "docs": [
              "Share of the pool in basis points (calculated on finalization)"
            ],
            "type": "u16"
          },
          {
            "name": "genesisNftMint",
            "docs": [
              "Genesis NFT mint address (set on finalization)"
            ],
            "type": "pubkey"
          },
          {
            "name": "feesClaimed",
            "docs": [
              "Total fees claimed"
            ],
            "type": "u64"
          },
          {
            "name": "nftMint",
            "docs": [
              "NFT mint address (if minted)"
            ],
            "type": {
              "option": "pubkey"
            }
          },
          {
            "name": "votingPowerBps",
            "docs": [
              "Voting power in BPS (set when NFT is minted)"
            ],
            "type": "u16"
          },
          {
            "name": "nftMinted",
            "docs": [
              "Whether NFT has been minted"
            ],
            "type": "bool"
          },
          {
            "name": "unwindClaimed",
            "docs": [
              "Whether investor has claimed unwind distribution"
            ],
            "type": "bool"
          },
          {
            "name": "refundClaimed",
            "docs": [
              "Whether investor has claimed failed bonding refund"
            ],
            "type": "bool"
          },
          {
            "name": "depositedAt",
            "docs": [
              "Timestamp of initial deposit"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "failedWithdrawal",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "feeMode",
      "docs": [
        "Fee distribution mode for Token Launcher"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "creatorRevenue"
          },
          {
            "name": "recoveryBoost"
          },
          {
            "name": "fairLaunch"
          }
        ]
      }
    },
    {
      "name": "feeThresholdRenounced",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "oldThresholdBps",
            "type": "u16"
          },
          {
            "name": "renouncedBy",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "feeThresholdUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "oldThresholdBps",
            "type": "u16"
          },
          {
            "name": "newThresholdBps",
            "type": "u16"
          }
        ]
      }
    },
    {
      "name": "feesClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "solFees",
            "type": "u64"
          },
          {
            "name": "tokenFees",
            "type": "u64"
          },
          {
            "name": "creatorShare",
            "type": "u64"
          },
          {
            "name": "investorShare",
            "type": "u64"
          },
          {
            "name": "protocolShare",
            "type": "u64"
          },
          {
            "name": "totalRecovered",
            "type": "u64"
          },
          {
            "name": "recoveryTarget",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "genesisNftMinted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "nftMint",
            "type": "pubkey"
          },
          {
            "name": "votingPowerBps",
            "type": "u16"
          },
          {
            "name": "depositAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "investorDeposited",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "depositorCount",
            "type": "u32"
          }
        ]
      }
    },
    {
      "name": "investorWithdrew",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "depositor",
            "type": "pubkey"
          },
          {
            "name": "amount",
            "type": "u64"
          },
          {
            "name": "remainingDeposit",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "permanentLock",
      "docs": [
        "Controls the Orca Whirlpool position NFT",
        "This PDA is the permanent delegate/owner of the position",
        "- Allows fee collection only",
        "- During Recovery: LP locked, can be unwound via governance",
        "- After Recovery: LP is PERMANENTLY LOCKED - no unwind possible"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereign",
            "docs": [
              "The sovereign this lock belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "whirlpool",
            "docs": [
              "The Whirlpool address"
            ],
            "type": "pubkey"
          },
          {
            "name": "positionMint",
            "docs": [
              "Position NFT mint address"
            ],
            "type": "pubkey"
          },
          {
            "name": "position",
            "docs": [
              "Position account address (PDA derived from position_mint)"
            ],
            "type": "pubkey"
          },
          {
            "name": "positionTokenAccount",
            "docs": [
              "Token account holding the position NFT"
            ],
            "type": "pubkey"
          },
          {
            "name": "liquidity",
            "docs": [
              "Total liquidity in the position"
            ],
            "type": "u128"
          },
          {
            "name": "tickLowerIndex",
            "docs": [
              "Lower tick index (always MIN_TICK for full range)"
            ],
            "type": "i32"
          },
          {
            "name": "tickUpperIndex",
            "docs": [
              "Upper tick index (always MAX_TICK for full range)"
            ],
            "type": "i32"
          },
          {
            "name": "unwound",
            "docs": [
              "Whether the LP has been unwound (only possible during Recovery)"
            ],
            "type": "bool"
          },
          {
            "name": "createdAt",
            "docs": [
              "Timestamp when position was created"
            ],
            "type": "i64"
          },
          {
            "name": "unwoundAt",
            "docs": [
              "Timestamp when unwound (0 if not unwound)"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "poolRestricted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "restricted",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "proposal",
      "docs": [
        "Unwind proposal during Recovery phase",
        "Only investors can create and vote (creator excluded)"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereign",
            "docs": [
              "The sovereign this proposal belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "proposalId",
            "docs": [
              "Unique proposal ID within the sovereign"
            ],
            "type": "u64"
          },
          {
            "name": "proposer",
            "docs": [
              "Address that created the proposal"
            ],
            "type": "pubkey"
          },
          {
            "name": "status",
            "docs": [
              "Current status"
            ],
            "type": {
              "defined": {
                "name": "proposalStatus"
              }
            }
          },
          {
            "name": "votesForBps",
            "docs": [
              "Total votes for (in basis points of total shares)"
            ],
            "type": "u32"
          },
          {
            "name": "votesAgainstBps",
            "docs": [
              "Total votes against (in basis points of total shares)"
            ],
            "type": "u32"
          },
          {
            "name": "totalVotedBps",
            "docs": [
              "Total participation (in basis points of total shares)"
            ],
            "type": "u32"
          },
          {
            "name": "voterCount",
            "docs": [
              "Number of unique voters"
            ],
            "type": "u32"
          },
          {
            "name": "quorumBps",
            "docs": [
              "Required quorum in basis points (default 6700 = 67%)"
            ],
            "type": "u16"
          },
          {
            "name": "passThresholdBps",
            "docs": [
              "Required pass threshold in basis points (default 5100 = 51%)"
            ],
            "type": "u16"
          },
          {
            "name": "votingEndsAt",
            "docs": [
              "Voting period end timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "timelockEndsAt",
            "docs": [
              "Timelock end timestamp (when execution is allowed)"
            ],
            "type": "i64"
          },
          {
            "name": "createdAt",
            "docs": [
              "Timestamp when proposal was created"
            ],
            "type": "i64"
          },
          {
            "name": "executedAt",
            "docs": [
              "Timestamp when proposal was executed (0 if not executed)"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "proposalCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "proposalId",
            "type": "u64"
          },
          {
            "name": "proposer",
            "type": "pubkey"
          },
          {
            "name": "createdAt",
            "type": "i64"
          },
          {
            "name": "votingEndsAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "proposalFinalized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "proposalId",
            "type": "u64"
          },
          {
            "name": "status",
            "type": {
              "defined": {
                "name": "proposalStatus"
              }
            }
          },
          {
            "name": "votesFor",
            "type": "u64"
          },
          {
            "name": "votesAgainst",
            "type": "u64"
          },
          {
            "name": "participationBps",
            "type": "u16"
          },
          {
            "name": "passed",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "proposalStatus",
      "docs": [
        "Status of a governance proposal"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "active"
          },
          {
            "name": "passed"
          },
          {
            "name": "failed"
          },
          {
            "name": "executed"
          },
          {
            "name": "cancelled"
          }
        ]
      }
    },
    {
      "name": "protocolFeesUpdated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "creationFeeBps",
            "type": "u16"
          },
          {
            "name": "minFeeLamports",
            "type": "u64"
          },
          {
            "name": "minDeposit",
            "type": "u64"
          },
          {
            "name": "minBondTarget",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "protocolInitialized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "type": "pubkey"
          }
        ]
      }
    },
    {
      "name": "protocolState",
      "docs": [
        "Protocol-level configuration and statistics",
        "Single PDA managing global settings for the Sovereign Liquidity Protocol"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "authority",
            "docs": [
              "Protocol admin - can update fees and settings"
            ],
            "type": "pubkey"
          },
          {
            "name": "treasury",
            "docs": [
              "Treasury wallet receiving protocol fees"
            ],
            "type": "pubkey"
          },
          {
            "name": "creationFeeBps",
            "docs": [
              "Creation fee in basis points (0-1000 = 0-10% of bond target)",
              "Default: 100 (1%)"
            ],
            "type": "u16"
          },
          {
            "name": "minFeeLamports",
            "docs": [
              "Minimum fee in lamports (non-refundable on failed bonding)",
              "Default: 0.05 SOL (50_000_000 lamports)"
            ],
            "type": "u64"
          },
          {
            "name": "governanceUnwindFeeLamports",
            "docs": [
              "Fee to create unwind proposal during recovery phase",
              "Default: 0.05 SOL (50_000_000 lamports)"
            ],
            "type": "u64"
          },
          {
            "name": "unwindFeeBps",
            "docs": [
              "Fee taken from SOL during unwind (0-1000 = 0-10%)",
              "Default: 500 (5%)"
            ],
            "type": "u16"
          },
          {
            "name": "byoMinSupplyBps",
            "docs": [
              "Minimum % of supply required for BYO Token launch",
              "Default: 3000 (30%)"
            ],
            "type": "u16"
          },
          {
            "name": "minBondTarget",
            "docs": [
              "Minimum bond target in lamports (50 SOL)"
            ],
            "type": "u64"
          },
          {
            "name": "minDeposit",
            "docs": [
              "Minimum single deposit in lamports (0.1 SOL)"
            ],
            "type": "u64"
          },
          {
            "name": "autoUnwindPeriod",
            "docs": [
              "Auto-unwind period in seconds (90-365 days)",
              "Protocol-controlled for Active phase activity check"
            ],
            "type": "i64"
          },
          {
            "name": "minFeeGrowthThreshold",
            "docs": [
              "Minimum fee growth to count as \"active\" (MUST be > 0)"
            ],
            "type": "u128"
          },
          {
            "name": "feeThresholdRenounced",
            "docs": [
              "If true, fee threshold is locked forever"
            ],
            "type": "bool"
          },
          {
            "name": "paused",
            "docs": [
              "Emergency pause flag"
            ],
            "type": "bool"
          },
          {
            "name": "sovereignCount",
            "docs": [
              "Total sovereigns created"
            ],
            "type": "u64"
          },
          {
            "name": "totalFeesCollected",
            "docs": [
              "Lifetime protocol revenue in lamports"
            ],
            "type": "u64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "recoveryComplete",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "totalRecovered",
            "type": "u64"
          },
          {
            "name": "recoveryTarget",
            "type": "u64"
          },
          {
            "name": "completedAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "sovereignCreated",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "creator",
            "type": "pubkey"
          },
          {
            "name": "tokenMint",
            "type": "pubkey"
          },
          {
            "name": "sovereignType",
            "type": {
              "defined": {
                "name": "sovereignType"
              }
            }
          },
          {
            "name": "bondTarget",
            "type": "u64"
          },
          {
            "name": "bondDeadline",
            "type": "i64"
          },
          {
            "name": "tokenSupplyDeposited",
            "type": "u64"
          },
          {
            "name": "creationFeeEscrowed",
            "type": "u64"
          },
          {
            "name": "sellFeeBps",
            "type": "u16"
          },
          {
            "name": "feeMode",
            "type": {
              "defined": {
                "name": "feeMode"
              }
            }
          }
        ]
      }
    },
    {
      "name": "sovereignFinalized",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "totalDeposited",
            "type": "u64"
          },
          {
            "name": "tokenSupply",
            "type": "u64"
          },
          {
            "name": "lpTokens",
            "type": "u64"
          },
          {
            "name": "recoveryTarget",
            "type": "u64"
          },
          {
            "name": "finalizedAt",
            "type": "i64"
          }
        ]
      }
    },
    {
      "name": "sovereignState",
      "docs": [
        "Main sovereign state account - one per token launch"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "docs": [
              "Unique identifier for this sovereign"
            ],
            "type": "u64"
          },
          {
            "name": "creator",
            "docs": [
              "Creator's wallet address"
            ],
            "type": "pubkey"
          },
          {
            "name": "tokenMint",
            "docs": [
              "Token mint address (created or BYO)"
            ],
            "type": "pubkey"
          },
          {
            "name": "sovereignType",
            "docs": [
              "Type of launch (TokenLaunch or BYOToken)"
            ],
            "type": {
              "defined": {
                "name": "sovereignType"
              }
            }
          },
          {
            "name": "state",
            "docs": [
              "Current lifecycle state"
            ],
            "type": {
              "defined": {
                "name": "sovereignStatus"
              }
            }
          },
          {
            "name": "bondTarget",
            "docs": [
              "Required SOL to raise (in lamports)"
            ],
            "type": "u64"
          },
          {
            "name": "bondDeadline",
            "docs": [
              "Unix timestamp deadline for bonding"
            ],
            "type": "i64"
          },
          {
            "name": "bondDuration",
            "docs": [
              "Duration in seconds (for reference)"
            ],
            "type": "i64"
          },
          {
            "name": "totalDeposited",
            "docs": [
              "Total SOL deposited by investors (excludes creator escrow)"
            ],
            "type": "u64"
          },
          {
            "name": "depositorCount",
            "docs": [
              "Number of unique investor depositors"
            ],
            "type": "u32"
          },
          {
            "name": "creatorEscrow",
            "docs": [
              "Creator's escrowed SOL for market buy (max 1% of bond target)"
            ],
            "type": "u64"
          },
          {
            "name": "tokenSupplyDeposited",
            "docs": [
              "Tokens deposited by creator (100% for TokenLaunch, >=30% for BYO)"
            ],
            "type": "u64"
          },
          {
            "name": "tokenTotalSupply",
            "docs": [
              "Total supply of token (for BYO verification)"
            ],
            "type": "u64"
          },
          {
            "name": "sellFeeBps",
            "docs": [
              "Sell fee in basis points (0-300 = 0-3%)"
            ],
            "type": "u16"
          },
          {
            "name": "feeMode",
            "docs": [
              "Fee distribution mode"
            ],
            "type": {
              "defined": {
                "name": "feeMode"
              }
            }
          },
          {
            "name": "feeControlRenounced",
            "docs": [
              "If true, creator cannot change sell_fee_bps"
            ],
            "type": "bool"
          },
          {
            "name": "creationFeeEscrowed",
            "docs": [
              "Amount held in creation fee escrow PDA"
            ],
            "type": "u64"
          },
          {
            "name": "whirlpool",
            "docs": [
              "Whirlpool address (set on finalization)"
            ],
            "type": "pubkey"
          },
          {
            "name": "positionMint",
            "docs": [
              "Position NFT mint (held by PermanentLock)"
            ],
            "type": "pubkey"
          },
          {
            "name": "poolRestricted",
            "docs": [
              "Whether pool restriction is active (LP locked to Genesis only)"
            ],
            "type": "bool"
          },
          {
            "name": "recoveryTarget",
            "docs": [
              "Target SOL to recover (equals total_deposited)"
            ],
            "type": "u64"
          },
          {
            "name": "totalSolFeesDistributed",
            "docs": [
              "Total SOL fees distributed to investors"
            ],
            "type": "u64"
          },
          {
            "name": "totalTokenFeesDistributed",
            "docs": [
              "Total token fees distributed to investors"
            ],
            "type": "u64"
          },
          {
            "name": "recoveryComplete",
            "docs": [
              "Whether recovery phase is complete"
            ],
            "type": "bool"
          },
          {
            "name": "activeProposalId",
            "docs": [
              "Active proposal ID (0 if none)"
            ],
            "type": "u64"
          },
          {
            "name": "proposalCount",
            "docs": [
              "Total proposals created"
            ],
            "type": "u64"
          },
          {
            "name": "hasActiveProposal",
            "docs": [
              "Whether there's an active proposal"
            ],
            "type": "bool"
          },
          {
            "name": "feeThresholdBps",
            "docs": [
              "Fee threshold in BPS (creator's share)"
            ],
            "type": "u16"
          },
          {
            "name": "totalFeesCollected",
            "docs": [
              "Total fees collected (for tracking)"
            ],
            "type": "u64"
          },
          {
            "name": "totalRecovered",
            "docs": [
              "Total recovered during recovery phase"
            ],
            "type": "u64"
          },
          {
            "name": "totalSupply",
            "docs": [
              "Total supply of tokens (for allocation)"
            ],
            "type": "u64"
          },
          {
            "name": "genesisNftMint",
            "docs": [
              "Genesis NFT collection mint"
            ],
            "type": "pubkey"
          },
          {
            "name": "unwoundAt",
            "docs": [
              "Timestamp when unwound (if applicable)"
            ],
            "type": {
              "option": "i64"
            }
          },
          {
            "name": "lastActivity",
            "docs": [
              "Last activity timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "activityCheckInitiated",
            "docs": [
              "Whether an activity check is in progress"
            ],
            "type": "bool"
          },
          {
            "name": "activityCheckInitiatedAt",
            "docs": [
              "Timestamp when activity check was initiated (Option for cleaner handling)"
            ],
            "type": {
              "option": "i64"
            }
          },
          {
            "name": "activityCheckTimestamp",
            "docs": [
              "Timestamp when activity check was initiated (legacy)"
            ],
            "type": "i64"
          },
          {
            "name": "feeGrowthSnapshotA",
            "docs": [
              "Snapshot of fee_growth_global_a at initiation"
            ],
            "type": "u128"
          },
          {
            "name": "feeGrowthSnapshotB",
            "docs": [
              "Snapshot of fee_growth_global_b at initiation"
            ],
            "type": "u128"
          },
          {
            "name": "activityCheckLastCancelled",
            "docs": [
              "Timestamp of last cancelled activity check (for cooldown)"
            ],
            "type": "i64"
          },
          {
            "name": "unwindSolBalance",
            "docs": [
              "SOL balance after removing liquidity (for claiming)"
            ],
            "type": "u64"
          },
          {
            "name": "unwindTokenBalance",
            "docs": [
              "Token balance after removing liquidity (for creator)"
            ],
            "type": "u64"
          },
          {
            "name": "lastActivityTimestamp",
            "docs": [
              "Last fee collection or activity timestamp"
            ],
            "type": "i64"
          },
          {
            "name": "createdAt",
            "docs": [
              "Timestamp when sovereign was created"
            ],
            "type": "i64"
          },
          {
            "name": "finalizedAt",
            "docs": [
              "Timestamp when finalized (LP created)"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "sovereignStatus",
      "docs": [
        "Current state of the sovereign lifecycle"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "bonding"
          },
          {
            "name": "finalizing"
          },
          {
            "name": "recovery"
          },
          {
            "name": "active"
          },
          {
            "name": "unwinding"
          },
          {
            "name": "unwound"
          },
          {
            "name": "failed"
          }
        ]
      }
    },
    {
      "name": "sovereignType",
      "docs": [
        "Type of token launch"
      ],
      "type": {
        "kind": "enum",
        "variants": [
          {
            "name": "tokenLaunch"
          },
          {
            "name": "byoToken"
          }
        ]
      }
    },
    {
      "name": "unwindClaimed",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "claimer",
            "type": "pubkey"
          },
          {
            "name": "solAmount",
            "type": "u64"
          },
          {
            "name": "tokenAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "unwindExecuted",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "executedAt",
            "type": "i64"
          },
          {
            "name": "solAmount",
            "type": "u64"
          },
          {
            "name": "tokenAmount",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "voteCast",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "sovereignId",
            "type": "u64"
          },
          {
            "name": "proposalId",
            "type": "u64"
          },
          {
            "name": "voter",
            "type": "pubkey"
          },
          {
            "name": "support",
            "type": "bool"
          },
          {
            "name": "votingPower",
            "type": "u64"
          },
          {
            "name": "votesFor",
            "type": "u64"
          },
          {
            "name": "votesAgainst",
            "type": "u64"
          }
        ]
      }
    },
    {
      "name": "voteRecord",
      "docs": [
        "Individual vote record to prevent double voting"
      ],
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proposal",
            "docs": [
              "The proposal this vote belongs to"
            ],
            "type": "pubkey"
          },
          {
            "name": "voter",
            "docs": [
              "The voter's wallet address"
            ],
            "type": "pubkey"
          },
          {
            "name": "genesisNftMint",
            "docs": [
              "Genesis NFT used for voting"
            ],
            "type": "pubkey"
          },
          {
            "name": "votingPowerBps",
            "docs": [
              "Voting power in basis points (from DepositRecord.shares_bps)"
            ],
            "type": "u16"
          },
          {
            "name": "voteFor",
            "docs": [
              "Whether voted for (true) or against (false)"
            ],
            "type": "bool"
          },
          {
            "name": "votedAt",
            "docs": [
              "Timestamp of vote"
            ],
            "type": "i64"
          },
          {
            "name": "bump",
            "docs": [
              "PDA bump seed"
            ],
            "type": "u8"
          }
        ]
      }
    }
  ]
};
