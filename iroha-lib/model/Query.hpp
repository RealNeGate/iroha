#pragma once

#include "LibsAndClassDeclarations.h"

#include "crypto/keypair.hpp"
#include "queries.pb.h"
#include "generators/QueryGenerator.hpp"


namespace iroha_lib {

class Query {

private:
    uint64_t counter_;
    uint64_t created_time_;
    iroha::protocol::Query protobuf_query_;
    iroha::keypair_t keypair_;
    QueryGenerator query_generator_;

public:
    Query(const iroha::keypair_t& keypair,
          uint64_t counter = 1u,
          uint64_t created_time = std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::system_clock::now().time_since_epoch()).count()) noexcept;

    Query& getAccount(const std::string& account_id);
    Query& getAccountAssets(const std::string& account_id);
    Query& getAccountDetail(const std::string& account_id);
    Query& getAccountTransactions(const std::string& account_id);
    Query& getAccountAssetTransactions(
            const std::string& account_id,
            const std::string& asset_id);
    Query& getTransactions(
            const std::string& account_id,
            const std::vector<std::string>& tx_hashes);
    Query& getSignatories(const std::string& account_id);
    Query& getAssetInfo(
            const std::string& account_id,
            const std::string& asset_id);
    Query& getRoles(const std::string& account_id);
    Query& getRolePermissions(
            const std::string& account_id,
            const std::string& role_id);

    const iroha::protocol::Query signAndAddSignature();
};

}  // namespace iroha_lib