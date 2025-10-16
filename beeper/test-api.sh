#!/bin/bash
# Beeper Desktop API Test Commands
# Use these to manually test the API and understand what the service sees

API_TOKEN=""
API_URL="http://localhost:23373"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${BLUE}Beeper Desktop API Test Commands${NC}"
echo "================================================="

# Test 1: Check API availability
echo -e "\n${YELLOW}1. Test API Availability${NC}"
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/search-chats?limit=1\""
echo
echo -e "${GREEN}Result:${NC}"
curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/search-chats?limit=1" | jq '.'
echo

# Test 2: Get all accounts
echo -e "\n${YELLOW}2. List Connected Accounts${NC}"
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/get-accounts\""
echo
echo -e "${GREEN}Result:${NC}"
curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/get-accounts" | jq -r '.[] | "\(.network) - \(.accountID)"'
echo

# Test 3: All unread messages (no time filter)
echo -e "\n${YELLOW}3. All Unread Messages (No Time Filter)${NC}"
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/search-chats?unreadOnly=true\""
echo
echo -e "${GREEN}Result:${NC}"
UNREAD_ALL=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/search-chats?unreadOnly=true")
echo "$UNREAD_ALL" | jq -r '.items[] | "[\(.network)] \(.title): \(.unreadCount) unread (last: \(.lastActivity))"'
TOTAL_ALL=$(echo "$UNREAD_ALL" | jq '.items | map(.unreadCount) | add // 0')
echo -e "${GREEN}Total unread (all time): $TOTAL_ALL${NC}"
echo

# Test 4: Recent unread messages (last 7 days - what the service uses)
echo -e "\n${YELLOW}4. Recent Unread Messages (Last 7 Days)${NC}"
DATE_7_DAYS_AGO=$(date -d "7 days ago" --utc +"%Y-%m-%dT%H:%M:%SZ")
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/search-chats?unreadOnly=true&lastActivityAfter=$DATE_7_DAYS_AGO\""
echo
echo -e "${GREEN}Result:${NC}"
UNREAD_RECENT=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/search-chats?unreadOnly=true&lastActivityAfter=$DATE_7_DAYS_AGO")
echo "$UNREAD_RECENT" | jq -r '.items[] | "[\(.network)] \(.title): \(.unreadCount) unread (last: \(.lastActivity))"'
TOTAL_RECENT=$(echo "$UNREAD_RECENT" | jq '.items | map(.unreadCount) | add // 0')
echo -e "${GREEN}Total unread (last 7 days): $TOTAL_RECENT${NC}"
echo

# Test 5: Custom time range
echo -e "\n${YELLOW}5. Custom Time Range (Last 30 Days)${NC}"
DATE_30_DAYS_AGO=$(date -d "30 days ago" --utc +"%Y-%m-%dT%H:%M:%SZ")
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/search-chats?unreadOnly=true&lastActivityAfter=$DATE_30_DAYS_AGO\""
echo
echo -e "${GREEN}Result:${NC}"
UNREAD_30DAYS=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/search-chats?unreadOnly=true&lastActivityAfter=$DATE_30_DAYS_AGO")
echo "$UNREAD_30DAYS" | jq -r '.items[] | "[\(.network)] \(.title): \(.unreadCount) unread (last: \(.lastActivity))"'
TOTAL_30DAYS=$(echo "$UNREAD_30DAYS" | jq '.items | map(.unreadCount) | add // 0')
echo -e "${GREEN}Total unread (last 30 days): $TOTAL_30DAYS${NC}"
echo

# Summary
echo -e "\n${BLUE}Summary${NC}"
echo "================================================="
echo -e "All time unread messages: ${GREEN}$TOTAL_ALL${NC}"
echo -e "Last 7 days (service uses): ${GREEN}$TOTAL_RECENT${NC}"
echo -e "Last 30 days: ${GREEN}$TOTAL_30DAYS${NC}"
echo
if [ "$TOTAL_RECENT" -gt 0 ]; then
    echo -e "${GREEN}LED should be blinking${NC} (recent unread messages found)"
else
    echo -e "${YELLOW}LED should be off${NC} (no recent unread messages)"
fi
echo

# Test 6: Query specific account
echo -e "\n${YELLOW}6. Query Specific Account (Example: Telegram)${NC}"
echo "Command:"
echo "curl -s -H \"Authorization: Bearer \$API_TOKEN\" \"$API_URL/v0/search-chats?accountIDs[]=telegram&unreadOnly=true\""
echo
echo -e "${GREEN}Result:${NC}"
curl -s -H "Authorization: Bearer $API_TOKEN" "$API_URL/v0/search-chats?accountIDs[]=telegram&unreadOnly=true" | jq -r '.items[] | "[Telegram] \(.title): \(.unreadCount) unread"'
echo

echo -e "\n${BLUE}Useful Commands for Testing${NC}"
echo "================================================="
echo "# Export your token for easier testing:"
echo "export BEEPER_TOKEN=\"$API_TOKEN\""
echo ""
echo "# Quick unread check:"
echo "curl -s -H \"Authorization: Bearer \$BEEPER_TOKEN\" \"$API_URL/v0/search-chats?unreadOnly=true&lastActivityAfter=\$(date -d '7 days ago' --utc +'%Y-%m-%dT%H:%M:%SZ')\" | jq '.items | map(.unreadCount) | add // 0'"
echo ""
echo "# Watch for changes (run in another terminal):"
echo "watch -n 5 \"curl -s -H 'Authorization: Bearer \$BEEPER_TOKEN' '$API_URL/v0/search-chats?unreadOnly=true' | jq '.items | map(.unreadCount) | add // 0'\""
echo ""
echo "# Check service logs:"
echo "journalctl --user -u beeper-led-blinker.service -f"
