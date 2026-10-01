import React from 'react'
import { createRoot } from 'react-dom/client'
import GroupMembersButton from '../../src/features/friends/GroupMembersButton'
import '../../src/styles/globals.css'

createRoot(document.getElementById('root')!).render(<GroupMembersButton groupId="fixture-group" count={16} />)
