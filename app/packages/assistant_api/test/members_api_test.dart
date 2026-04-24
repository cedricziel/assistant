import 'package:test/test.dart';
import 'package:assistant_api/assistant_api.dart';

/// tests for MembersApi
void main() {
  final instance = AssistantApi().getMembersApi();

  group(MembersApi, () {
    // `POST /api/orgs/{org_id}/spaces/{space_id}/members` — add a member.
    //
    //Future<MemberEntry> addMember(String orgId, String spaceId, AddMemberRequest addMemberRequest) async
    test('test addMember', () async {
      // TODO
    });

    // `GET /api/orgs/{org_id}/spaces/{space_id}/members` — list space members.
    //
    //Future<BuiltList<MemberEntry>> listMembers(String orgId, String spaceId) async
    test('test listMembers', () async {
      // TODO
    });

    // `DELETE /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — remove member.
    //
    //Future removeMember(String orgId, String spaceId, String userId) async
    test('test removeMember', () async {
      // TODO
    });

    // `PATCH /api/orgs/{org_id}/spaces/{space_id}/members/{user_id}` — change role.
    //
    //Future<MemberEntry> updateMember(String orgId, String spaceId, String userId, UpdateMemberRequest updateMemberRequest) async
    test('test updateMember', () async {
      // TODO
    });
  });
}
